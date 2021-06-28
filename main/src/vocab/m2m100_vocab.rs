// Copyright 2021 The Fairseq Authors and The HuggingFace Inc. team. All rights reserved.
// Copyright 2019-2021 Guillaume Becquin
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//     http://www.apache.org/licenses/LICENSE-2.0
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use crate::error::TokenizerError;
use crate::vocab::base_vocab::swap_key_values;
use crate::vocab::Vocab;
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::BufReader;

pub static FAIRSEQ_LANGUAGE_CODES: [&str; 100] = [
    "af", "am", "ar", "ast", "az", "ba", "be", "bg", "bn", "br", "bs", "ca", "ceb", "cs", "cy",
    "da", "de", "el", "en", "es", "et", "fa", "ff", "fi", "fr", "fy", "ga", "gd", "gl", "gu", "ha",
    "he", "hi", "hr", "ht", "hu", "hy", "id", "ig", "ilo", "is", "it", "ja", "jv", "ka", "kk",
    "km", "kn", "ko", "lb", "lg", "ln", "lo", "lt", "lv", "mg", "mk", "ml", "mn", "mr", "ms", "my",
    "ne", "nl", "no", "ns", "oc", "or", "pa", "pl", "ps", "pt", "ro", "ru", "sd", "si", "sk", "sl",
    "so", "sq", "sr", "ss", "su", "sv", "sw", "ta", "th", "tl", "tn", "tr", "uk", "ur", "uz", "vi",
    "wo", "xh", "yi", "yo", "zh", "zu",
];

/// # M2M100 Vocab
/// Vocabulary for M2M100 tokenizer. Contains the following special values:
/// - PAD token
/// - BOS token
/// - EOS token
/// - SEP token
///
/// Expects a JSON-format vocabulary when created from file.
#[allow(clippy::upper_case_acronyms)]
#[derive(Debug, Clone)]
pub struct M2M100Vocab {
    /// A mapping of tokens as string to indices (i.e. the encoder base)
    pub values: HashMap<String, i64>,

    /// A mapping of token IDs to strings (i.e. the decoder base)
    pub indices: HashMap<i64, String>,

    /// The string to use for unknown (out of vocabulary) tokens
    pub unknown_value: &'static str,

    /// A mapping of special value tokens as strings to IDs (i.e. the encoder base for special
    /// values), special values typically include things like BOS/EOS markers, class markers, mask
    /// markers and padding markers
    pub special_values: HashMap<String, i64>,

    /// A mapping of special value tokens as IDs to strings (i.e. the decoder base for special values)
    pub special_indices: HashMap<i64, String>,

    /// Language code stored as bytes for extraction of the prefix in input sequences
    pub language_codes_bytes: HashSet<Vec<u8>>,
}

impl Vocab for M2M100Vocab {
    fn unknown_value() -> &'static str {
        "<unk>"
    }

    fn get_unknown_value(&self) -> &'static str {
        "<unk>"
    }

    fn pad_value() -> Option<&'static str> {
        Some("<pad>")
    }

    fn sep_value() -> Option<&'static str> {
        Some("</s>")
    }

    fn cls_value() -> Option<&'static str> {
        None
    }

    fn mask_value() -> Option<&'static str> {
        None
    }

    fn bos_value() -> Option<&'static str> {
        Some("<s>")
    }

    fn eos_value() -> Option<&'static str> {
        Some("</s>")
    }

    fn values(&self) -> &HashMap<String, i64> {
        &self.values
    }

    fn indices(&self) -> &HashMap<i64, String> {
        &self.indices
    }

    fn special_values(&self) -> &HashMap<String, i64> {
        &self.special_values
    }

    fn special_indices(&self) -> &HashMap<i64, String> {
        &self.special_indices
    }

    fn from_file(path: &str) -> Result<M2M100Vocab, TokenizerError> {
        let f = File::open(path).map_err(|e| {
            TokenizerError::FileNotFound(format!("{} vocabulary file not found :{}", path, e))
        })?;
        let br = BufReader::new(f);
        let mut values: HashMap<String, i64> = match serde_json::from_reader(br) {
            Ok(value) => value,
            Err(e) => {
                return Err(TokenizerError::VocabularyParsingError(e.to_string()));
            }
        };

        for language_code in FAIRSEQ_LANGUAGE_CODES.iter() {
            values.insert(
                if language_code.len() == 2 {
                    format!(">>{}.<<", language_code)
                } else if language_code.len() == 3 {
                    format!(">>{}<<", language_code)
                } else {
                    return Err(TokenizerError::VocabularyParsingError(
                        "M2M100 Vocab only supports language code of length 2 or 3".to_string(),
                    ));
                },
                values.len() as i64,
            );
        }

        let mut special_values = HashMap::new();
        let unknown_value = M2M100Vocab::unknown_value();
        M2M100Vocab::_register_as_special_value(unknown_value, &values, &mut special_values)?;

        let sep_value = M2M100Vocab::sep_value().unwrap();
        M2M100Vocab::_register_as_special_value(sep_value, &values, &mut special_values)?;

        let bos_value = M2M100Vocab::bos_value().unwrap();
        M2M100Vocab::_register_as_special_value(bos_value, &values, &mut special_values)?;

        let eos_value = M2M100Vocab::eos_value().unwrap();
        M2M100Vocab::_register_as_special_value(eos_value, &values, &mut special_values)?;

        let pad_value = M2M100Vocab::pad_value().unwrap();
        M2M100Vocab::_register_as_special_value(pad_value, &values, &mut special_values)?;

        let indices = swap_key_values(&values);
        let special_indices = swap_key_values(&special_values);
        let language_codes_bytes = FAIRSEQ_LANGUAGE_CODES
            .iter()
            .map(|f| {
                if f.len() == 2 {
                    format!(">>{}.<<", f)
                } else {
                    format!(">>{}<<", f)
                }
                .as_bytes()
                .to_vec()
            })
            .collect::<HashSet<Vec<u8>>>();

        Ok(M2M100Vocab {
            values,
            indices,
            unknown_value,
            special_values,
            special_indices,
            language_codes_bytes,
        })
    }

    fn token_to_id(&self, token: &str) -> i64 {
        self._token_to_id(
            token,
            &self.values,
            &self.special_values,
            &self.unknown_value,
        )
    }

    fn id_to_token(&self, id: &i64) -> String {
        self._id_to_token(
            &id,
            &self.indices,
            &self.special_indices,
            &self.unknown_value,
        )
    }
}
