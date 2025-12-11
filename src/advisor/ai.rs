use crate::stat::datatype::*;
use crate::stat::{Ledger, timephase_fromnow};
use anyhow::{Result, anyhow};
use candle_core::quantized::gguf_file;
use candle_core::{Device, Tensor};
use candle_transformers::generation::{LogitsProcessor, Sampling};

use candle_transformers::models::quantized_qwen2::ModelWeights as Qwen2;
use hf_hub::api::sync::Api;
use hf_hub::{Repo, RepoType};
use std::path::PathBuf;
use tokenizers::Tokenizer;
pub const TOOL: &str = r#"
{"name":"top_spend_recent",
 "description":"Get top spending categories over the last N months, based on outcome (negative amounts).",
 "parameters":{
   "type":"object",
   "properties":{
     "months":{
       "type":"integer",
       "description":"How many recent months to look back, counting current month as 1.",
       "minimum":1,
       "maximum":24
     },
     "top_k":{
       "type":"integer",
       "description":"How many top categories to return.",
       "minimum":1,
       "maximum":10
     }
   },
   "required":["months"]
 }}
{"name":"month_total_spend",
 "description":"Get total spending for a given year and month (all accounts, all categories).",
 "parameters":{
   "type":"object",
   "properties":{
     "year":{
       "type":"integer",
       "description":"4-digit year, e.g. 2025."
     },
     "month":{
       "type":"integer",
       "description":"Calendar month 1-12.",
       "minimum":1,
       "maximum":12
     }
   },
   "required":["year","month"]
 }}
{"name":"month_top_category",
 "description":"Get the top spending categories for a given year and month and their share of total spending.",
 "parameters":{
   "type":"object",
   "properties":{
     "year":{"type":"integer"},
     "month":{
       "type":"integer",
       "minimum":1,
       "maximum":12
     },
     "top_k":{
       "type":"integer",
       "description":"How many categories to return.",
       "minimum":1,
       "maximum":10
     }
   },
   "required":["year","month"]
 }}
{"name":"add_simple_expense",
 "description":"Add a single expense entry to the local ledger (best-effort helper).",
 "parameters":{
   "type":"object",
   "properties":{
     "date":{
       "type":"string",
       "description":"Date in YYYY-MM-DD format."
     },
     "account_name":{
       "type":"string",
       "description":"Account name, e.g. 'Chequing' or 'Visa'."
     },
     "category_name":{
       "type":"string",
       "description":"Category name such as 'Food' or 'Rent'."
     },
     "amount":{
       "type":"number",
       "description":"Expense amount in CAD, positive number."
     },
     "description":{
       "type":"string",
       "description":"Optional short memo."
     }
   },
   "required":["date","account_name","category_name","amount"]
 }}
"#;
fn _device() -> Device {
    #[cfg(feature = "cuda")]
    {
        let cuda = Device::new_cuda(0);
        match cuda {
            Ok(c) => {
                println!("find cuda");
                return c;
            }
            Err(_) => {}
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
#[derive(Debug, Clone, Copy)]
pub enum Modeltype {
    Qwen25_1_5B,
    Qwen25_0_5B,
    Qwen25_3B,
    Qwen25_7B,
}

impl Modeltype {
    fn tok_address(self) -> &'static str {
        match self {
            Modeltype::Qwen25_7B => "Qwen/Qwen2.5-7B-Instruct",
            Modeltype::Qwen25_0_5B => "Qwen/Qwen2.5-0.5B-Instruct",
            Modeltype::Qwen25_1_5B => "Qwen/Qwen2.5-1.5B-Instruct",
            Modeltype::Qwen25_3B => "Qwen/Qwen2.5-3B-Instruct",
        }
    }
    fn gguf_address(self) -> (&'static str, &'static str) {
        match self {
            Modeltype::Qwen25_7B => (
                "Qwen/Qwen2.5-7B-Instruct-GGUF",
                "qwen2.5-7b-instruct-q3_k_m.gguf",
            ),
            Modeltype::Qwen25_0_5B => (
                "Qwen/Qwen2.5-0.5B-Instruct-GGUF",
                "qwen2.5-0.5b-instruct-q6_k.gguf",
            ),
            Modeltype::Qwen25_1_5B => (
                "Qwen/Qwen2.5-1.5B-Instruct-GGUF",
                "qwen2.5-1.5b-instruct-q6_k.gguf",
            ),
            Modeltype::Qwen25_3B => (
                "Qwen/Qwen2.5-3B-Instruct-GGUF",
                "qwen2.5-3b-instruct-q4_k_m.gguf",
            ),
        }
    }
    fn eos(self) -> &'static str {
        match self {
            Modeltype::Qwen25_7B
            | Modeltype::Qwen25_3B
            | Modeltype::Qwen25_1_5B
            | Modeltype::Qwen25_0_5B => "<|im_end|>",
        }
    }
    fn apply_chat_template(&self, user: &str) -> String {
        let mut template = String::new();
        template.push_str("<|im_start|>system\nYou are a personal finance assistant.<|im_end|>\n<|im_start|>user\n");
        template.push_str(user);
        template.push_str("<|im_end|>\n<|im_start|>assistant\n");
        template
    }
    fn apply_into_tool_chat_template(&self, user: &str, functionintro: &str) -> String {
        let mut template = String::new();
        template.push_str("<|im_start|>system\nYou are a personal finance assistant.");
        template.push_str("You may call one or more functions to assist with the user query.\n\nYou are provided with function signatures within <tools></tools> XML tags:\n<tools>\n",);
        template.push_str(functionintro);
        template.push_str("\n</tools>\n\nFor each function call, return a json object with function name and arguments within <tool_call></tool_call> XML tags:\n<tool_call>\n{\"name\": <function-name>, \"arguments\": <args-json-object>}\n</tool_call><|im_end|>\n",);
        template.push_str("<|im_start|>user\n");
        template.push_str(user);
        template.push_str("<|im_end|>\n<|im_start|>assistant\n");
        template
    }
    fn apply_tool_out_chat_template(&self, premessage: &str, functionresult: &str) -> String {
        let mut template = String::new();
        template.push_str(premessage);
        template.push_str("<|im_end|>\n<|im_start|>user\n<tool_response>\n");
        template.push_str(functionresult);
        template.push_str("\n</tool_response><|im_end|>\n<|im_start|>assistant\n");
        template
    }
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
    pub weight: Qwen2,
    pub device: Device,
    pub name: Modeltype,
}
impl Model {
    pub fn checklocal(model_type: Modeltype) -> Result<Localmodel> {
        let api = match Api::new() {
            Ok(i) => i,
            Err(_) => {
                return Err(anyhow!(
                    "can't connect to Hugging Face, please check Code or Internet"
                ));
            }
        };
        let tok_file = api.model(model_type.tok_address().to_string());
        let tok = match tok_file.get("tokenizer.json") {
            Ok(t) => t,
            Err(_) => return Err(anyhow!("can't find tokenizer file")),
        };
        let gguf_file = api.repo(Repo::with_revision(
            model_type.gguf_address().0.to_string(),
            RepoType::Model,
            "main".to_string(),
        ));
        let gguf = match gguf_file.get(model_type.gguf_address().1) {
            Ok(t) => t,
            Err(_) => return Err(anyhow!("can't find model file")),
        };

        Ok(Localmodel {
            name: model_type.tok_address().to_string(),
            tokenizer: tok,
            gguf: gguf,
        })
    }
    pub fn new_with(model_type: Modeltype) -> Result<Self> {
        let file = match Self::checklocal(model_type) {
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
        let weight = Qwen2::from_gguf(temp, &mut f, &device)?;

        Ok(Self {
            localfile: file,
            tokenizer: tok,
            weight: weight,
            device: device,
            name: model_type,
        })
    }
    pub fn new() -> Result<Self> {
        Self::new_with(Modeltype::Qwen25_1_5B)
    }
    //https://github.com/huggingface/candle/blob/main/candle-examples/examples/quantized-qwen2-instruct/main.rs
    pub fn generation(
        &mut self,
        content: &str,
        cfg: &Generationcfg,
        usetool: Option<bool>,
    ) -> Result<String> {
        let txt = if usetool.unwrap_or(false) {
            self.name.apply_into_tool_chat_template(content, TOOL)
        } else {
            self.name.apply_chat_template(content)
        };

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
            .get(self.name.eos())
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
        let cad1 = self.generation(&prompt, cfg, None)?;
        let cad2 = self.generation(&prompt, cfg, None)?;
        let mut result = Vec::new();
        result.push(prompt);
        result.push(cad1);
        result.push(cad2);
        return Ok(result);
    }
}
