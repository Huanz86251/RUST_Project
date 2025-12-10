use crate::stat::datatype::{AccountId, CategoryId, UserId};
use crate::stat::{Ledger, timephase_fromnow};
use anyhow::{Context, Result, anyhow};
use candle_core::quantized::gguf_file;
use candle_core::{Device, Tensor};
use candle_transformers::generation::{LogitsProcessor, Sampling};
use candle_transformers::models::quantized_qwen2::ModelWeights;
use chrono::{Datelike, Utc};
use hf_hub::api::sync::Api;
use hf_hub::{Repo, RepoType};
use std::fmt::Write as FmtWrite;
use std::path::PathBuf;
use tokenizers::Tokenizer;
fn _device() -> Device {
    #[cfg(feature = "cuda")]
    {
        let cuda = Device::new_cuda(0);
        match cuda {
            Ok(c) => {
                println!("find cuda");
                return c;
            }
            Err(e) => {
                eprintln!("⚠ cuda init failed: {e}");
            }
        }
    }
    #[cfg(feature = "metal")]
    {
        let metal = Device::new_metal(0);
        match metal {
            Ok(c) => {
                println!("find metal");
                return c;
            }
            Err(_) => {}
        }
    }
    println!("device= cpu");
    Device::Cpu
}
#[derive(Debug, Clone)]
pub struct Localmodel {
    pub name: String,
    pub tokenizer: PathBuf,
    pub gguf: PathBuf,
}

#[derive(Debug, Clone)]
pub struct Generationcfg {
    pub max_new_tok: usize,
    pub temperature: f64,
    pub top_p: f64,
}

impl Default for Generationcfg {
    fn default() -> Self {
        Self {
            max_new_tok: 256,
            temperature: 0.7,
            top_p: 0.9,
        }
    }
}
pub struct Model {
    pub localfile: Localmodel,
    pub tokenizer: Tokenizer,
    pub weight: ModelWeights,
    pub device: Device,
}
impl Model {
    pub fn checklocal() -> Result<Localmodel> {
        let api = match Api::new() {
            Ok(i) => i,
            Err(_) => {
                return Err(anyhow!(
                    "can't connect to Hugging Face, please check Code or Internet"
                ));
            }
        };
        let tok_file = api.model("Qwen/Qwen2.5-1.5B-Instruct".to_string());
        let tok = match tok_file.get("tokenizer.json") {
            Ok(t) => t,
            Err(_) => return Err(anyhow!("can't find tokenizer file")),
        };
        let gguf_file = api.repo(Repo::with_revision(
            "Qwen/Qwen2.5-1.5B-Instruct-GGUF".to_string(),
            RepoType::Model,
            "main".to_string(),
        ));
        let gguf = match gguf_file.get("qwen2.5-1.5b-instruct-q6_k.gguf") {
            Ok(t) => t,
            Err(_) => return Err(anyhow!("can't find model file")),
        };

        Ok(Localmodel {
            name: "Qwen/Qwen2.5-1.5B-Instruct".to_string(),
            tokenizer: tok,
            gguf: gguf,
        })
    }
    pub fn new() -> Result<Self> {
        let file = match Self::checklocal() {
            Ok(t) => t,
            Err(e) => return Err(anyhow!("{}", e)),
        };
        let tok = match Tokenizer::from_file(&file.tokenizer) {
            Ok(t) => t,
            Err(_) => return Err(anyhow!("can't load tokenizer file")),
        };

        let device = _device();
        let mut f = std::fs::File::open(&file.gguf)?;
        let temp = gguf_file::Content::read(&mut f)?;
        let weight = ModelWeights::from_gguf(temp, &mut f, &device)?;

        Ok(Self {
            localfile: file,
            tokenizer: tok,
            weight: weight,
            device: device,
        })
    }
    //https://github.com/huggingface/candle/blob/main/candle-examples/examples/quantized-qwen2-instruct/main.rs
    pub fn generation(&mut self, content: &str, cfg: &Generationcfg) -> Result<String> {
        let mut txt = String::new();
        txt.push_str("<|im_start|>user\n");
        txt.push_str(content);
        txt.push_str("\n<|im_end|>\n<|im_start|>assistant\n");
        let txt_tok = self.tokenizer.encode(txt, true).unwrap();
        let mut tok = txt_tok.get_ids().to_vec();
        let pre_len = tok.len();
        let samp = if cfg.temperature <= 0.0 {
            Sampling::ArgMax
        } else {
            Sampling::TopP {
                p: cfg.top_p,
                temperature: cfg.temperature,
            }
        };
        let seed: u64 = rand::random();
        let mut processor = LogitsProcessor::from_sampling(seed, samp);
        let prefill = Tensor::new(&tok[..], &self.device)?.unsqueeze(0)?;
        let pre_log = self.weight.forward(&prefill, 0)?.squeeze(0)?;
        let mut next_tok = processor.sample(&pre_log)?;
        tok.push(next_tok);
        let eso = self
            .tokenizer
            .get_vocab(true)
            .get("<|im_end|>")
            .map_or(0, |id| *id);
        let mut if_end = false;
        for i in 0..(cfg.max_new_tok - 1) {
            if next_tok == eso {
                if_end = true;
                break;
            }

            let temp = Tensor::new(&[next_tok], &self.device)?.unsqueeze(0)?;
            let logit = self.weight.forward(&temp, pre_len + i)?.squeeze(0)?;
            next_tok = processor.sample(&logit)?;
            tok.push(next_tok);
        }
        if if_end {
            tok.pop();
        }
        let out_tok = tok[pre_len..].to_vec();
        let output = self.tokenizer.decode(&out_tok, true).unwrap();
        Ok(output)
    }
    pub fn build_prompt(
        &self,
        ledger: &Ledger,
        userid: UserId,
        top_k: usize,
        pastmonths: u32,
    ) -> String {
        let timephase = timephase_fromnow(pastmonths);
        let trend = ledger.data_linetrend(userid, timephase, None, None);
        let lm = timephase.1.1;
        let ly = timephase.1.0;
        let top_cat = ledger.top_category(userid, ((ly, lm), (ly, lm)), None, top_k, Some(true));
        let mut prompt = String::new();
        prompt.push_str("You are a personal finance assistant.\n");
        prompt.push_str("Recent monthly totals:\n");
        for i in 0..trend.axis.len() {
            let y = trend.axis[i].0;
            let m = trend.axis[i].1;
            let sum = trend.summary[i];
            if sum < 0.0 {
                let spend = -sum;
                prompt.push_str(&format!(
                    "- {y:04}-{m:02}: spend {spend:.2} CAD\n",
                    y = y,
                    m = m,
                    spend = spend
                ));
            } else {
                prompt.push_str(&format!(
                    "- {y:04}-{m:02}: total {sum:.2} CAD\n",
                    y = y,
                    m = m,
                    sum = sum
                ));
            }
        }
        prompt.push_str(&format!(
            "Main spending categories in {ly:04}-{lm:02}:\n",
            ly = ly,
            lm = lm
        ));
        for i in 0..top_cat.axis.len() {
            let name = &top_cat.axis[i];
            let out = top_cat.outcome[i].abs();
            prompt.push_str(&format!("- {name}: {out:.2} CAD\n", name = name, out = out));
        }
        prompt.push_str("\n");
        prompt.push_str(
            "Now, in less than 200 English words:\n\
1) Explain the main risks or problems in my spending.\n\
2) Give 2–3 specific suggestions to improve my situation.\n\
Focus on categories and behaviours, not on exact amounts.\n",
        );
        prompt
    }
    pub fn generate_advicepair(
        &mut self,
        ledger: &Ledger,
        userid: UserId,
        top_k: usize,
        pastmonths: u32,
        cfg: &Generationcfg,
    ) -> Result<Vec<String>> {
        let prompt = self.build_prompt(ledger, userid, top_k, pastmonths);
        let cad1 = self.generation(&prompt, cfg)?;
        let cad2 = self.generation(&prompt, cfg)?;
        let mut result = Vec::new();
        result.push(prompt);
        result.push(cad1);
        result.push(cad2);
        return Ok(result);
    }
}
