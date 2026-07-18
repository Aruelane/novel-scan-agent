use std::fmt;

use crate::NovelFormat;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ImportError {
    PendingSupport { format: NovelFormat, detail: String },
    UnsupportedFormat { source_name: String, detail: String },
    EncodingPending { encoding: String, detail: String },
    InvalidText { encoding: String, detail: String },
    EmptyDocument { source_name: String },
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
        }
    }
}

impl std::error::Error for ImportError {}
