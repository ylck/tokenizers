extern crate tokenizers as tk;

use super::utils::Container;
use crate::tokenizer::AddedToken;
use pyo3::exceptions;
use pyo3::prelude::*;
use pyo3::types::*;

#[pyclass]
pub struct Trainer {
    pub trainer: Container<dyn tk::tokenizer::Trainer>,
}

#[pyclass(extends=Trainer)]
pub struct BpeTrainer {}
#[pymethods]
impl BpeTrainer {
    /// new(/ vocab_size, min_frequency)
    /// --
    ///
    /// Create a new BpeTrainer with the given configuration
    #[new]
    #[args(kwargs = "**")]
    pub fn new(obj: &PyRawObject, kwargs: Option<&PyDict>) -> PyResult<()> {
        let mut builder = tk::models::bpe::BpeTrainer::builder();
        if let Some(kwargs) = kwargs {
            for (key, val) in kwargs {
                let key: &str = key.extract()?;
                match key {
                    "vocab_size" => builder = builder.vocab_size(val.extract()?),
                    "min_frequency" => builder = builder.min_frequency(val.extract()?),
                    "show_progress" => builder = builder.show_progress(val.extract()?),
                    "special_tokens" => {
                        builder = builder.special_tokens(
                            val.cast_as::<PyList>()?
                                .into_iter()
                                .map(|token| {
                                    if let Ok(content) = token.extract::<String>() {
                                        Ok(tk::tokenizer::AddedToken {
                                            content,
                                            ..Default::default()
                                        })
                                    } else if let Ok(token) = token.cast_as::<AddedToken>() {
                                        Ok(token.token.clone())
                                    } else {
                                        Err(exceptions::Exception::py_err(
                                            "special_tokens must be a List[Union[str, AddedToken]]",
                                        ))
                                    }
                                })
                                .collect::<PyResult<Vec<_>>>()?,
                        );
                    }
                    "limit_alphabet" => builder = builder.limit_alphabet(val.extract()?),
                    "initial_alphabet" => {
                        let alphabet: Vec<String> = val.extract()?;
                        builder = builder.initial_alphabet(
                            alphabet
                                .into_iter()
                                .map(|s| s.chars().nth(0))
                                .filter(|c| c.is_some())
                                .map(|c| c.unwrap())
                                .collect(),
                        );
                    }
                    "continuing_subword_prefix" => {
                        builder = builder.continuing_subword_prefix(val.extract()?)
                    }
                    "end_of_word_suffix" => builder = builder.end_of_word_suffix(val.extract()?),
                    _ => println!("Ignored unknown kwargs option {}", key),
                };
            }
        }
        Ok(obj.init(Trainer {
            trainer: Container::Owned(Box::new(builder.build())),
        }))
    }
}

#[pyclass(extends=Trainer)]
pub struct WordPieceTrainer {}
#[pymethods]
impl WordPieceTrainer {
    /// new(/ vocab_size, min_frequency)
    /// --
    ///
    /// Create a new BpeTrainer with the given configuration
    #[new]
    #[args(kwargs = "**")]
    pub fn new(obj: &PyRawObject, kwargs: Option<&PyDict>) -> PyResult<()> {
        let mut builder = tk::models::wordpiece::WordPieceTrainer::builder();
        if let Some(kwargs) = kwargs {
            for (key, val) in kwargs {
                let key: &str = key.extract()?;
                match key {
                    "vocab_size" => builder = builder.vocab_size(val.extract()?),
                    "min_frequency" => builder = builder.min_frequency(val.extract()?),
                    "show_progress" => builder = builder.show_progress(val.extract()?),
                    "special_tokens" => {
                        builder = builder.special_tokens(
                            val.cast_as::<PyList>()?
                                .into_iter()
                                .map(|token| {
                                    if let Ok(content) = token.extract::<String>() {
                                        Ok(tk::tokenizer::AddedToken {
                                            content,
                                            ..Default::default()
                                        })
                                    } else if let Ok(token) = token.cast_as::<AddedToken>() {
                                        Ok(token.token.clone())
                                    } else {
                                        Err(exceptions::Exception::py_err(
                                            "special_tokens must be a List[Union[str, AddedToken]]",
                                        ))
                                    }
                                })
                                .collect::<PyResult<Vec<_>>>()?,
                        );
                    }
                    "limit_alphabet" => builder = builder.limit_alphabet(val.extract()?),
                    "initial_alphabet" => {
                        let alphabet: Vec<String> = val.extract()?;
                        builder = builder.initial_alphabet(
                            alphabet
                                .into_iter()
                                .map(|s| s.chars().nth(0))
                                .filter(|c| c.is_some())
                                .map(|c| c.unwrap())
                                .collect(),
                        );
                    }
                    "continuing_subword_prefix" => {
                        builder = builder.continuing_subword_prefix(val.extract()?)
                    }
                    "end_of_word_suffix" => builder = builder.end_of_word_suffix(val.extract()?),
                    _ => println!("Ignored unknown kwargs option {}", key),
                };
            }
        }

        Ok(obj.init(Trainer {
            trainer: Container::Owned(Box::new(builder.build())),
        }))
    }
}
