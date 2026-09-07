/// A small span-preserving JSON reader for value homes (§FS-values.2.2,
/// §REQ-never-crashes.1). It preserves ordered and duplicate object members,
/// raw scalar spellings, decoded strings, and byte spans before catalog maps
/// are built; syntax failures remain scan-incomplete rather than semantic value
/// findings.

#[derive(Clone, Copy, Debug)]
struct JsonSpan {
    start: usize,
    end: usize,
}

#[derive(Debug)]
struct JsonStringValue {
    decoded: String,
    span: JsonSpan,
}

#[derive(Debug)]
struct JsonMember {
    key: JsonStringValue,
    value: JsonNode,
    span: JsonSpan,
}

#[derive(Debug)]
enum JsonNode {
    Object(Vec<JsonMember>, JsonSpan),
    Array(Vec<JsonNode>, JsonSpan),
    String(JsonStringValue),
    Number(String, JsonSpan),
    InvalidScalar(JsonSpan),
    Bool(JsonSpan),
    Null(JsonSpan),
}

impl JsonNode {
    fn span(&self) -> JsonSpan {
        match self {
            Self::Object(_, span)
            | Self::Array(_, span)
            | Self::Number(_, span)
            | Self::InvalidScalar(span)
            | Self::Bool(span)
            | Self::Null(span) => *span,
            Self::String(value) => value.span,
        }
    }
}

struct JsonReader<'a> {
    text: &'a str,
    pos: usize,
}

impl<'a> JsonReader<'a> {
    fn parse(text: &'a str) -> std::result::Result<JsonNode, String> {
        let mut reader = Self { text, pos: 0 };
        reader.ws();
        let value = reader.value()?;
        reader.ws();
        if reader.pos != text.len() {
            return Err(reader.error("unexpected content after JSON value"));
        }
        Ok(value)
    }

    fn value(&mut self) -> std::result::Result<JsonNode, String> {
        self.ws();
        match self.peek() {
            Some(b'{') => self.object(),
            Some(b'[') => self.array(),
            Some(b'"') => self.string().map(JsonNode::String),
            Some(b't') => self.keyword("true", JsonNode::Bool),
            Some(b'f') => self.keyword("false", JsonNode::Bool),
            Some(b'n') => self.keyword("null", JsonNode::Null),
            Some(b'-' | b'+' | b'0'..=b'9') => Ok(self.number()),
            Some(b'N' | b'I') => Ok(self.invalid_scalar()),
            Some(_) => Err(self.error("expected a JSON value")),
            None => Err(self.error("expected a JSON value")),
        }
    }

    fn object(&mut self) -> std::result::Result<JsonNode, String> {
        let start = self.pos;
        self.pos += 1;
        self.ws();
        let mut members = Vec::new();
        if self.take(b'}') {
            return Ok(JsonNode::Object(members, JsonSpan { start, end: self.pos }));
        }
        loop {
            self.ws();
            let member_start = self.pos;
            let key = self.string()?;
            self.ws();
            self.expect(b':', "expected `:` after object key")?;
            let value = self.value()?;
            let member_end = value.span().end;
            members.push(JsonMember {
                key,
                value,
                span: JsonSpan {
                    start: member_start,
                    end: member_end,
                },
            });
            self.ws();
            if self.take(b'}') {
                break;
            }
            self.expect(b',', "expected `,` or `}` after object member")?;
            self.ws();
            if self.peek() == Some(b'}') {
                return Err(self.error("trailing comma in object"));
            }
        }
        Ok(JsonNode::Object(
            members,
            JsonSpan {
                start,
                end: self.pos,
            },
        ))
    }

    fn array(&mut self) -> std::result::Result<JsonNode, String> {
        let start = self.pos;
        self.pos += 1;
        self.ws();
        let mut values = Vec::new();
        if self.take(b']') {
            return Ok(JsonNode::Array(values, JsonSpan { start, end: self.pos }));
        }
        loop {
            values.push(self.value()?);
            self.ws();
            if self.take(b']') {
                break;
            }
            self.expect(b',', "expected `,` or `]` after array element")?;
            self.ws();
            if self.peek() == Some(b']') {
                return Err(self.error("trailing comma in array"));
            }
        }
        Ok(JsonNode::Array(
            values,
            JsonSpan {
                start,
                end: self.pos,
            },
        ))
    }

    fn string(&mut self) -> std::result::Result<JsonStringValue, String> {
        let start = self.pos;
        self.expect(b'"', "expected JSON string")?;
        let mut decoded = String::new();
        while let Some(byte) = self.peek() {
            match byte {
                b'"' => {
                    self.pos += 1;
                    return Ok(JsonStringValue {
                        decoded,
                        span: JsonSpan {
                            start,
                            end: self.pos,
                        },
                    });
                }
                b'\\' => {
                    self.pos += 1;
                    self.escape(&mut decoded)?;
                }
                0x00..=0x1f => return Err(self.error("control character in JSON string")),
                _ => {
                    let rest = &self.text[self.pos..];
                    let ch = rest.chars().next().ok_or_else(|| self.error("invalid UTF-8"))?;
                    decoded.push(ch);
                    self.pos += ch.len_utf8();
                }
            }
        }
        Err(self.error("unterminated JSON string"))
    }

    fn escape(&mut self, out: &mut String) -> std::result::Result<(), String> {
        let escaped = self.peek().ok_or_else(|| self.error("unterminated JSON escape"))?;
        self.pos += 1;
        match escaped {
            b'"' => out.push('"'),
            b'\\' => out.push('\\'),
            b'/' => out.push('/'),
            b'b' => out.push('\u{0008}'),
            b'f' => out.push('\u{000c}'),
            b'n' => out.push('\n'),
            b'r' => out.push('\r'),
            b't' => out.push('\t'),
            b'u' => {
                let first = self.hex_quad()?;
                let scalar = if (0xd800..=0xdbff).contains(&first) {
                    if !self.text[self.pos..].starts_with("\\u") {
                        return Err(self.error("high surrogate without low surrogate"));
                    }
                    self.pos += 2;
                    let second = self.hex_quad()?;
                    if !(0xdc00..=0xdfff).contains(&second) {
                        return Err(self.error("invalid low surrogate"));
                    }
                    0x10000 + (((first - 0xd800) as u32) << 10) + (second - 0xdc00) as u32
                } else if (0xdc00..=0xdfff).contains(&first) {
                    return Err(self.error("low surrogate without high surrogate"));
                } else {
                    first as u32
                };
                out.push(char::from_u32(scalar).ok_or_else(|| self.error("invalid Unicode escape"))?);
            }
            _ => return Err(self.error("invalid JSON escape")),
        }
        Ok(())
    }

    fn hex_quad(&mut self) -> std::result::Result<u16, String> {
        let end = self.pos.saturating_add(4);
        let raw = self
            .text
            .get(self.pos..end)
            .ok_or_else(|| self.error("incomplete Unicode escape"))?;
        if !raw.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(self.error("invalid Unicode escape"));
        }
        self.pos = end;
        u16::from_str_radix(raw, 16).map_err(|_| self.error("invalid Unicode escape"))
    }

    fn number(&mut self) -> JsonNode {
        let start = self.pos;
        while self
            .peek()
            .is_some_and(|byte| !matches!(byte, b' ' | b'\n' | b'\r' | b'\t' | b',' | b']' | b'}'))
        {
            self.pos += 1;
        }
        let raw = &self.text[start..self.pos];
        if !JSON_NUMBER_RE.is_match(raw) {
            return JsonNode::InvalidScalar(JsonSpan {
                start,
                end: self.pos,
            });
        }
        JsonNode::Number(
            raw.to_string(),
            JsonSpan {
                start,
                end: self.pos,
            },
        )
    }

    fn invalid_scalar(&mut self) -> JsonNode {
        let start = self.pos;
        while self
            .peek()
            .is_some_and(|byte| !matches!(byte, b' ' | b'\n' | b'\r' | b'\t' | b',' | b']' | b'}'))
        {
            self.pos += 1;
        }
        JsonNode::InvalidScalar(JsonSpan {
            start,
            end: self.pos,
        })
    }

    fn keyword(
        &mut self,
        word: &str,
        make: fn(JsonSpan) -> JsonNode,
    ) -> std::result::Result<JsonNode, String> {
        let start = self.pos;
        if !self.text[self.pos..].starts_with(word) {
            return Err(self.error("invalid JSON literal"));
        }
        self.pos += word.len();
        Ok(make(JsonSpan {
            start,
            end: self.pos,
        }))
    }

    fn ws(&mut self) {
        while self
            .peek()
            .is_some_and(|byte| matches!(byte, b' ' | b'\n' | b'\r' | b'\t'))
        {
            self.pos += 1;
        }
    }

    fn peek(&self) -> Option<u8> {
        self.text.as_bytes().get(self.pos).copied()
    }

    fn take(&mut self, expected: u8) -> bool {
        if self.peek() == Some(expected) {
            self.pos += 1;
            true
        } else {
            false
        }
    }

    fn expect(&mut self, expected: u8, message: &str) -> std::result::Result<(), String> {
        if self.take(expected) {
            Ok(())
        } else {
            Err(self.error(message))
        }
    }

    fn error(&self, message: &str) -> String {
        let (line, column) = json_line_column(self.text, self.pos);
        format!("{line}:{column}: {message}")
    }
}

fn json_line_column(text: &str, byte: usize) -> (usize, usize) {
    let prefix = &text[..byte.min(text.len())];
    let line = prefix.bytes().filter(|byte| *byte == b'\n').count() + 1;
    let line_start = prefix.rfind('\n').map_or(0, |index| index + 1);
    let column = prefix[line_start..].chars().count() + 1;
    (line, column)
}
