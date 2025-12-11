//base on offical Candle repository https://github.com/huggingface/candle/blob/main/candle-examples/examples/quantized-qwen2-instruct/main.rs, with own build
use crate::stat::datatype::*;
use crate::stat::{Ledger, timephase_fromnow};
use anyhow::{Result, anyhow};
use candle_core::quantized::gguf_file;
use candle_core::{Device, Tensor};
use candle_transformers::generation::{LogitsProcessor, Sampling};
use candle_transformers::models::quantized_qwen2::ModelWeights as Qwen2;
use chrono::{Datelike, Local};
use hf_hub::api::sync::Api;
use hf_hub::{Repo, RepoType};
use serde::Deserialize;
use serde_json::Value as JsonValue;
use std::path::PathBuf;
use tokenizers::Tokenizer;
pub const TOOL: &str = r#"
{"name":"month_summary",
 "description":"Get totals for a specific month, or for a range of months, over all accounts and categories. Use this whenever the user asks how much they spent, earned, or the net total over one or more months.",
 "parameters":{
   "type":"object",
   "properties":{
     "year":{
       "type":"integer",
       "description":"Start year, 4-digit, e.g. 2025."
     },
     "month":{
       "type":"integer",
       "description":"Start month 1-12.",
       "minimum":1,
       "maximum":12
     },
     "end_year":{
       "type":"integer",
       "description":"Optional end year, 4-digit. If omitted, defaults to the same as `year`."
     },
     "end_month":{
       "type":"integer",
       "description":"Optional end month 1-12. If omitted, defaults to the same as `month`.",
       "minimum":1,
       "maximum":12
     },
     "kind":{
       "type":"string",
       "description":"Which total to compute: 'spend' for total spending, 'income' for total income, 'net' for income minus spending. If omitted, default is 'spend'.",
       "enum":["spend","income","net"]
     }
   },
   "required":["year","month"]
 }}
{"name":"recent_top_category",
 "description":"Get the top spending categories over the last N months, counting current month as 1. Use this when the user asks which categories they spend the most on recently.",
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
       "description":"How many categories to return.",
       "minimum":1,
       "maximum":10
     }
   },
   "required":["months"]
 }}
{"name":"recent_top_account",
 "description":"Get the accounts with the highest spending over the last N months, counting current month as 1. Use this when the user asks which card or account they spend the most.",
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
       "description":"How many accounts to return.",
       "minimum":1,
       "maximum":10
     }
   },
   "required":["months"]
 }}
 {"name":"recent_trend",
  "description":"Get month-by-month totals (income, spending, and net) over the last N months, counting current month as 1. Use this when the user asks whether their spending is going up or down over time.",
  "parameters":{
    "type":"object",
    "properties":{
      "months":{
        "type":"integer",
        "description":"How many recent months to look back, counting current month as 1.If the user does not specify the number of months, choose 3 months by default.",
        "minimum":1,
        "maximum":24
      }
    },
    "required":["months"]
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
#[derive(Debug, Deserialize)]
pub struct Toolcall {
    pub name: String,
    pub arguments: JsonValue,
}
fn extract_fun(raw: &str) -> Option<Toolcall> {
    let start = "<tool_call>";
    let end = "</tool_call>";
    let start_pos = match raw.find(start) {
        Some(pos) => pos + start.len(),
        None => return None,
    };
    let end_pos = match raw.find(end) {
        Some(pos) => pos,
        None => return None,
    };
    let slice_raw = &raw[start_pos..end_pos];
    let json_raw = slice_raw.trim();
    if json_raw.is_empty() {
        return None;
    }
    let json: serde_json::Result<Toolcall> = serde_json::from_str(json_raw);
    let toolcall = match json {
        Ok(v) => v,
        Err(_) => {
            return None;
        }
    };
    return Some(toolcall);
}

fn tool_month_summary(ledger: &Ledger, userid: UserId, args: &JsonValue) -> String {
    let s_y = match args.get("year") {
        Some(v) => match v.as_i64() {
            Some(i) => i as i32,
            None => return "given parameter wrong, function give error.".to_string(),
        },
        None => return "given parameter wrong, function give error.".to_string(),
    };
    let e_y = match args.get("end_year") {
        Some(v) => match v.as_i64() {
            Some(i) => i as i32,
            None => return "given parameter wrong, function give error.".to_string(),
        },
        None => s_y,
    };
    let s_m = match args.get("month") {
        Some(v) => match v.as_i64() {
            Some(i) => i as u32,
            None => return "given parameter wrong, function give error.".to_string(),
        },
        None => return "given parameter wrong, function give error.".to_string(),
    };
    let e_m = match args.get("end_month") {
        Some(v) => match v.as_i64() {
            Some(i) => i as u32,
            None => return "given parameter wrong, function give error.".to_string(),
        },
        None => s_m,
    };
    let ((sy, sm), (ey, em)) = if (e_y, e_m) > (s_y, s_m) {
        ((s_y, s_m), (e_y, e_m))
    } else {
        ((e_y, e_m), (s_y, s_m))
    };
    let timephase = Some(((sy, sm), (ey, em)));
    let kind = match args.get("kind") {
        Some(i) => i.as_str().unwrap_or("spend"),
        None => "spend",
    };
    match kind {
        "spend" => {
            let sum = ledger.month_summary(userid, sy, sm, None, None, Some(true), timephase);
            let spend = -sum;
            format!(
                "Total spending from {sy:04}-{sm:02} to {ey:04}-{em:02} is {spend:.2} CAD.",
                sy = sy,
                sm = sm,
                ey = ey,
                em = em,
                spend = spend
            )
        }
        "net" => {
            let sum = ledger.month_summary(userid, sy, sm, None, None, None, timephase);

            format!(
                "Total net income/outcome from {sy:04}-{sm:02} to {ey:04}-{em:02} is {spend:.2} CAD.",
                sy = sy,
                sm = sm,
                ey = ey,
                em = em,
                spend = sum
            )
        }
        "income" => {
            let sum = ledger.month_summary(userid, sy, sm, None, None, Some(false), timephase);

            format!(
                "Total income from {sy:04}-{sm:02} to {ey:04}-{em:02} is {spend:.2} CAD.",
                sy = sy,
                sm = sm,
                ey = ey,
                em = em,
                spend = sum
            )
        }
        _ => "unknown kind type, must be: spend / net / income".to_string(),
    }
}
fn tool_recent_top_category(ledger: &Ledger, userid: UserId, args: &JsonValue) -> String {
    let m = match args.get("months") {
        Some(v) => match v.as_i64() {
            Some(i) => i as u32,
            None => return "given parameter wrong, function give error.".to_string(),
        },
        None => return "given parameter wrong, function give error.".to_string(),
    };
    let k = match args.get("top_k") {
        Some(v) => match v.as_u64() {
            Some(i) => i as usize,
            None => return "given parameter wrong, function give error.".to_string(),
        },
        None => 5 as usize,
    };
    let timephase = timephase_fromnow(m);
    let trend = ledger.top_category(userid, timephase, None, k, Some(true));
    let mut out = String::new();
    for (cat, val) in trend.axis.iter().zip(trend.outcome.iter()) {
        let spend = val.abs();
        out.push_str(&format!(
            "- {cat}:{spend:.2}CAD\n",
            cat = cat,
            spend = spend
        ));
    }
    out
}
fn tool_recent_top_account(ledger: &Ledger, userid: UserId, args: &JsonValue) -> String {
    let m = match args.get("months") {
        Some(v) => match v.as_i64() {
            Some(i) => i as u32,
            None => return "given parameter wrong, function give error.".to_string(),
        },
        None => return "given parameter wrong, function give error.".to_string(),
    };
    let k = match args.get("top_k") {
        Some(v) => match v.as_u64() {
            Some(i) => i as usize,
            None => return "given parameter wrong, function give error.".to_string(),
        },
        None => 5 as usize,
    };
    let timephase = timephase_fromnow(m);
    let trend = ledger.top_account(userid, timephase, None, k, Some(true));
    let mut out = String::new();
    for (cat, val) in trend.axis.iter().zip(trend.outcome.iter()) {
        let spend = val.abs();
        out.push_str(&format!(
            "- {cat}:{spend:.2}CAD\n",
            cat = cat,
            spend = spend
        ));
    }
    out
}
fn tool_recent_trend(ledger: &Ledger, userid: UserId, args: &JsonValue) -> String {
    let mon = match args.get("months") {
        Some(v) => match v.as_i64() {
            Some(i) => i as u32,
            None => return "given parameter wrong, function give error.".to_string(),
        },
        None => return "given parameter wrong, function give error.".to_string(),
    };
    let timephase = timephase_fromnow(mon);
    let trend = ledger.data_linetrend(userid, timephase, None, None);
    let mut s = String::new();
    s.push_str("Monthly trend: ");
    for i in 0..trend.axis.len() {
        let y = trend.axis[i].0;
        let m = trend.axis[i].1;
        let inc = trend.income[i];
        let mut out = trend.outcome[i];
        out = out.abs();
        let sum = trend.summary[i];
        s.push_str(&format!(
            "- {y:04}-{m:02}: income {inc:.2}, spend {out:.2}, total {sum:.2}CAD\n",
            y = y,
            m = m,
            inc = inc,
            out = out,
            sum = sum
        ));
    }
    s
}
fn run_toolcall(toolcall: &Toolcall, ledger: &Ledger, userid: UserId) -> String {
    match toolcall.name.as_str() {
        "month_summary" => tool_month_summary(ledger, userid, &toolcall.arguments),
        "recent_top_account" => tool_recent_top_account(ledger, userid, &toolcall.arguments),
        "recent_top_category" => tool_recent_top_category(ledger, userid, &toolcall.arguments),
        "recent_trend" => tool_recent_trend(ledger, userid, &toolcall.arguments),
        //"add_simple_expense" => tool_add_simple_expense(ledger, userid, &toolcall.arguments),
        _ => "unknown tool name".to_string(),
    }
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
        template.push_str("You must call exactly one function that best matches the user query.\nDo not call more than one function.\n\nYou are provided with function signatures within <tools></tools> XML tags:\n<tools>\n",);
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
        template.push_str("\n</tool_response>Please analyze these results and give 2–3 positive, practical suggestions for my finances.<|im_end|>\n<|im_start|>assistant\n");
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
    pub fn generation_core(&mut self, content: &str, cfg: &Generationcfg) -> Result<String> {
        let txt_tok = self.tokenizer.encode(content, true).unwrap();
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
    pub fn generation(&mut self, content: &str, cfg: &Generationcfg) -> Result<String> {
        let txt = self.name.apply_chat_template(content);

        self.generation_core(&txt, cfg)
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
    pub fn answer_withtool(
        &mut self,
        content: &str,
        ledger: &Ledger,
        userid: UserId,
        cfg: &Generationcfg,
    ) -> Result<String> {
        let now = Local::now();
        let year = now.year();
        let month = now.month();
        let time_con = format!(
            "Today is {year:04}-{month:02}. {content}",
            year = year,
            month = month,
            content = content
        );
        let prompt_first = self.name.apply_into_tool_chat_template(&time_con, TOOL);
        let first_turn = match self.generation_core(&prompt_first, cfg) {
            Ok(v) => v,
            Err(e) => return Err(e),
        };
        let fc = match extract_fun(&first_turn) {
            Some(f) => f,
            None => {
                let backup = match self.generation(&time_con, cfg) {
                    Ok(v) => v,
                    Err(e) => return Err(e),
                };
                return Ok(backup);
            }
        };
        let r = run_toolcall(&fc, ledger, userid);
        let second_prompt = self.name.apply_tool_out_chat_template(&prompt_first, &r);
        let final_a = match self.generation_core(&second_prompt, cfg) {
            Ok(v) => v,
            Err(e) => return Err(e),
        };
        Ok(final_a)
    }
}
