use std::fmt;

use crate::NovelFormat;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ImportError {
    PendingSupport {
        format: NovelFormat,
        detail: String,
    },
    UnsupportedFormat {
        source_name: String,
        detail: String,
    },
    EncodingPending {
        encoding: String,
        detail: String,
    },
    InvalidText {
        encoding: String,
        detail: String,
    },
    EmptyDocument {
        source_name: String,
    },
    Corrupt {
        source_name: String,
        detail: String,
    },
    LimitExceeded {
        source_name: String,
        limit: String,
        detail: String,
    },
    OcrRequired {
        source_name: String,
        detail: String,
    },
    ConversionRequired {
        source_name: String,
        detail: String,
    },
    Protected {
        source_name: String,
        detail: String,
    },
}

impl fmt::Display for ImportError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PendingSupport { format, detail } => {
                write!(formatter, "{} 导入尚未实现：{detail}", format.stable_id())
            }
            Self::UnsupportedFormat {
                source_name,
                detail,
            } => write!(formatter, "不支持 {source_name}：{detail}"),
            Self::EncodingPending { encoding, detail } => {
                write!(formatter, "{encoding} 解码尚未实现：{detail}")
            }
            Self::InvalidText { encoding, detail } => {
                write!(formatter, "{encoding} 文本无效：{detail}")
            }
            Self::EmptyDocument { source_name } => {
                write!(formatter, "{source_name} 没有可导入的正文")
            }
            Self::Corrupt {
                source_name,
                detail,
            } => {
                write!(formatter, "{source_name} 文件损坏：{detail}")
            }
            Self::LimitExceeded {
                source_name,
                limit,
                detail,
            } => {
                write!(formatter, "{source_name} 超过{limit}限制：{detail}")
            }
            Self::OcrRequired {
                source_name,
                detail,
            } => {
                write!(formatter, "{source_name} 需要 OCR：{detail}")
            }
            Self::ConversionRequired {
                source_name,
                detail,
            } => {
                write!(formatter, "{source_name} 需要转换：{detail}")
            }
            Self::Protected {
                source_name,
                detail,
            } => {
                write!(formatter, "{source_name} 受保护：{detail}")
            }
        }
    }
}

impl std::error::Error for ImportError {}
