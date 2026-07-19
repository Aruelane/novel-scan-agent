use crate::ImportError;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TextEncoding {
    Utf8,
    Utf8Bom,
    Utf16Le,
    Utf16Be,
    Gbk,
    Gb18030,
}

impl TextEncoding {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Utf8 => "UTF-8",
            Self::Utf8Bom => "UTF-8 BOM",
            Self::Utf16Le => "UTF-16 LE",
            Self::Utf16Be => "UTF-16 BE",
            Self::Gbk => "GBK",
            Self::Gb18030 => "GB18030",
        }
    }
}

/// Optional user-provided encoding override.
///
/// GBK and GB18030 are represented even though the decoder is not bundled yet,
/// so callers receive an actionable pending result rather than mojibake.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TextEncodingHint {
    Utf8,
    Utf16Le,
    Utf16Be,
    Gbk,
    Gb18030,
}

impl TextEncodingHint {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Utf8 => "UTF-8",
            Self::Utf16Le => "UTF-16 LE",
            Self::Utf16Be => "UTF-16 BE",
            Self::Gbk => "GBK",
            Self::Gb18030 => "GB18030",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DecodedText {
    pub text: String,
    pub encoding: TextEncoding,
}

pub(crate) fn decode(
    bytes: &[u8],
    hint: Option<TextEncodingHint>,
) -> Result<DecodedText, ImportError> {
    if let Some(hint) = hint {
        return decode_with_hint(bytes, hint);
    }

    if let Some(payload) = bytes.strip_prefix(&[0xef, 0xbb, 0xbf]) {
        return decode_utf8(payload, TextEncoding::Utf8Bom);
    }
    if let Some(payload) = bytes.strip_prefix(&[0xff, 0xfe]) {
        return decode_utf16(payload, true);
    }
    if let Some(payload) = bytes.strip_prefix(&[0xfe, 0xff]) {
        return decode_utf16(payload, false);
    }
    if let Ok(text) = std::str::from_utf8(bytes) {
        return Ok(DecodedText {
            text: text.to_owned(),
            encoding: TextEncoding::Utf8,
        });
    }

    Err(ImportError::EncodingPending {
        encoding: "未知旧式编码（可能是 GBK/GB18030）".to_owned(),
        detail:
            "当前内置解码器不会猜测并产生乱码；请选择正确编码，或等待 GBK/GB18030 跨端解码器接入"
                .to_owned(),
    })
}

fn decode_with_hint(bytes: &[u8], hint: TextEncodingHint) -> Result<DecodedText, ImportError> {
    match hint {
        TextEncodingHint::Utf8 => {
            let payload = bytes.strip_prefix(&[0xef, 0xbb, 0xbf]).unwrap_or(bytes);
            let encoding = if payload.len() == bytes.len() {
                TextEncoding::Utf8
            } else {
                TextEncoding::Utf8Bom
            };
            decode_utf8(payload, encoding)
        }
        TextEncodingHint::Utf16Le => {
            let payload = bytes.strip_prefix(&[0xff, 0xfe]).unwrap_or(bytes);
            decode_utf16(payload, true)
        }
        TextEncodingHint::Utf16Be => {
            let payload = bytes.strip_prefix(&[0xfe, 0xff]).unwrap_or(bytes);
            decode_utf16(payload, false)
        }
        TextEncodingHint::Gbk => decode_gbk(bytes),
        TextEncodingHint::Gb18030 => decode_gb18030(bytes),
    }
}

fn decode_utf8(bytes: &[u8], encoding: TextEncoding) -> Result<DecodedText, ImportError> {
    let text = std::str::from_utf8(bytes).map_err(|error| ImportError::InvalidText {
        encoding: encoding.label().to_owned(),
        detail: format!("第 {} 个字节附近不是有效的 UTF-8", error.valid_up_to()),
    })?;
    Ok(DecodedText {
        text: text.to_owned(),
        encoding,
    })
}

fn decode_utf16(bytes: &[u8], little_endian: bool) -> Result<DecodedText, ImportError> {
    let encoding = if little_endian {
        TextEncoding::Utf16Le
    } else {
        TextEncoding::Utf16Be
    };

    if bytes.len() % 2 != 0 {
        return Err(ImportError::InvalidText {
            encoding: encoding.label().to_owned(),
            detail: "UTF-16 数据长度不是 2 字节的整数倍".to_owned(),
        });
    }

    let units = bytes
        .chunks_exact(2)
        .map(|pair| {
            if little_endian {
                u16::from_le_bytes([pair[0], pair[1]])
            } else {
                u16::from_be_bytes([pair[0], pair[1]])
            }
        })
        .collect::<Vec<_>>();
    let text = String::from_utf16(&units).map_err(|error| ImportError::InvalidText {
        encoding: encoding.label().to_owned(),
        detail: format!("UTF-16 包含无效代理项：{error}"),
    })?;

    Ok(DecodedText { text, encoding })
}

fn decode_gbk(bytes: &[u8]) -> Result<DecodedText, ImportError> {
    let (text, _encoding, had_errors) = encoding_rs::GBK.decode(bytes);
    if had_errors {
        return Err(ImportError::InvalidText {
            encoding: "GBK".to_owned(),
            detail: "包含无法映射的字节序列".to_owned(),
        });
    }
    Ok(DecodedText {
        text: text.into_owned(),
        encoding: TextEncoding::Gbk,
    })
}

fn decode_gb18030(bytes: &[u8]) -> Result<DecodedText, ImportError> {
    let (text, _encoding, had_errors) = encoding_rs::GB18030.decode(bytes);
    if had_errors {
        return Err(ImportError::InvalidText {
            encoding: "GB18030".to_owned(),
            detail: "包含无法映射的字节序列".to_owned(),
        });
    }
    Ok(DecodedText {
        text: text.into_owned(),
        encoding: TextEncoding::Gb18030,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_utf8_bom_without_leaking_the_marker() {
        let decoded = decode(b"\xef\xbb\xbfhello", None).unwrap();
        assert_eq!(decoded.text, "hello");
        assert_eq!(decoded.encoding, TextEncoding::Utf8Bom);
    }

    #[test]
    fn decodes_utf16_little_endian_bom() {
        let text = "第一章\r\n正文";
        let mut bytes = vec![0xff, 0xfe];
        for unit in text.encode_utf16() {
            bytes.extend_from_slice(&unit.to_le_bytes());
        }

        let decoded = decode(&bytes, None).unwrap();
        assert_eq!(decoded.text, text);
        assert_eq!(decoded.encoding, TextEncoding::Utf16Le);
    }

    #[test]
    fn decodes_gbk_with_explicit_hint() {
        // GBK encoding of "第一章"
        let gbk_bytes = [0xb5, 0xda, 0xd2, 0xbb, 0xd5, 0xc2];
        let decoded = decode(&gbk_bytes, Some(TextEncodingHint::Gbk)).unwrap();
        assert_eq!(decoded.text, "第一章");
        assert_eq!(decoded.encoding, TextEncoding::Gbk);
    }

    #[test]
    fn reports_legacy_chinese_encoding_as_pending_without_hint() {
        let error = decode(&[0xb5, 0xda, 0xd2, 0xbb, 0xd5, 0xc2], None).unwrap_err();
        assert!(matches!(error, ImportError::EncodingPending { .. }));
    }
}
