/// Exact component classification and equality for first-class values
/// (§FS-values.2, §FS-values.4). Decimals stay as normalized coefficient and
/// arbitrary-size decimal exponent strings; no binary float or exponent
/// expansion is involved.

#[derive(Debug, Clone)]
pub enum DeclarationSource {
    Text,
    Json {
        member_slice: String,
        key_column: usize,
        key_text: String,
    },
}

/// One authoritative component shared by Markdown and JSON value declarations
/// (§FS-values.2, §FS-values.4).
#[derive(Debug, Clone)]
pub struct ValueComponent {
    pub decoded: String,
    pub kind: ValueComponentKind,
    pub source_slice: String,
    pub column: usize,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum ValueComponentKind {
    Number,
    String,
}

/// An exact authored binding beside the ordinary citation the scanner emits
/// for the same token (§FS-values.3, §AR-scanner.3).
#[derive(Debug)]
pub struct ValueBinding {
    pub namespace: Option<String>,
    pub id: Id,
    pub section: String,
    pub authored: ValueComponent,
    pub file: PathBuf,
    pub line: usize,
    pub column: usize,
}

/// A readable declaration or attempted binding that violates the explicit
/// value grammar (§FS-values.5.2, §AR-scanner.3).
#[derive(Debug)]
pub struct InvalidValueSite {
    pub id: Option<Id>,
    pub file: PathBuf,
    pub line: usize,
    pub column: Option<usize>,
    pub message: String,
    pub source: DeclarationSource,
}

static JSON_NUMBER_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^-?(?:0|[1-9][0-9]*)(?:\.[0-9]+)?(?:[eE][+-]?[0-9]+)?$").unwrap()
});

fn kind_uses_values(config: &Config, kind: &str) -> bool {
    config
        .kinds
        .iter()
        .any(|configured| configured.kind == kind && configured.values)
}

fn component_text_is_valid(text: &str) -> bool {
    !text.is_empty()
        && text.trim() == text
        && !text.contains('`')
        && !text.chars().any(char::is_control)
}

fn authored_component(text: &str, column: usize) -> ValueComponent {
    ValueComponent {
        decoded: text.to_string(),
        kind: if JSON_NUMBER_RE.is_match(text) {
            ValueComponentKind::Number
        } else {
            ValueComponentKind::String
        },
        source_slice: text.to_string(),
        column,
    }
}

fn value_components_equal(left: &ValueComponent, right: &ValueComponent) -> bool {
    match (left.kind, right.kind) {
        (ValueComponentKind::Number, ValueComponentKind::Number) => {
            ExactDecimal::parse(&left.decoded) == ExactDecimal::parse(&right.decoded)
        }
        (ValueComponentKind::String, ValueComponentKind::String) => left.decoded == right.decoded,
        _ => false,
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ExactDecimal {
    negative: bool,
    coefficient: String,
    exponent: SignedDigits,
}

impl ExactDecimal {
    fn parse(raw: &str) -> Option<Self> {
        if !JSON_NUMBER_RE.is_match(raw) {
            return None;
        }
        let (negative, unsigned) = raw
            .strip_prefix('-')
            .map_or((false, raw), |rest| (true, rest));
        let (mantissa, exponent_raw) = unsigned
            .split_once(['e', 'E'])
            .map_or((unsigned, "0"), |(mantissa, exponent)| (mantissa, exponent));
        let (whole, fraction) = mantissa
            .split_once('.')
            .map_or((mantissa, ""), |(whole, fraction)| (whole, fraction));
        let mut coefficient = format!("{whole}{fraction}");
        let first_nonzero = coefficient.find(|ch| ch != '0').unwrap_or(coefficient.len());
        coefficient.drain(..first_nonzero);
        if coefficient.is_empty() {
            return Some(Self {
                negative: false,
                coefficient: "0".to_string(),
                exponent: SignedDigits::zero(),
            });
        }
        let trailing = coefficient.len() - coefficient.trim_end_matches('0').len();
        coefficient.truncate(coefficient.len() - trailing);
        let exponent = SignedDigits::parse(exponent_raw)?
            .add_signed_usize(false, fraction.len())
            .add_signed_usize(true, trailing);
        Some(Self {
            negative,
            coefficient,
            exponent,
        })
    }
}

/// Sign plus normalized base-10 magnitude. Only addition/subtraction by an
/// input-length-sized `usize` is needed to account for a decimal point and
/// stripped coefficient zeroes, so exponent length remains unbounded.
#[derive(Clone, Debug, Eq, PartialEq)]
struct SignedDigits {
    negative: bool,
    digits: String,
}

impl SignedDigits {
    fn zero() -> Self {
        Self {
            negative: false,
            digits: "0".to_string(),
        }
    }

    fn parse(raw: &str) -> Option<Self> {
        let (negative, digits) = if let Some(rest) = raw.strip_prefix('-') {
            (true, rest)
        } else if let Some(rest) = raw.strip_prefix('+') {
            (false, rest)
        } else {
            (false, raw)
        };
        if digits.is_empty() || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
            return None;
        }
        let digits = digits.trim_start_matches('0');
        if digits.is_empty() {
            return Some(Self::zero());
        }
        Some(Self {
            negative,
            digits: digits.to_string(),
        })
    }

    fn add_signed_usize(mut self, positive: bool, amount: usize) -> Self {
        if amount == 0 {
            return self;
        }
        let amount = amount.to_string();
        if self.digits == "0" {
            self.negative = !positive;
            self.digits = amount;
            return self;
        }
        if self.negative == !positive {
            self.digits = add_magnitudes(&self.digits, &amount);
            return self;
        }
        match compare_magnitudes(&self.digits, &amount) {
            std::cmp::Ordering::Greater => {
                self.digits = subtract_magnitudes(&self.digits, &amount);
            }
            std::cmp::Ordering::Less => {
                self.digits = subtract_magnitudes(&amount, &self.digits);
                self.negative = !self.negative;
            }
            std::cmp::Ordering::Equal => return Self::zero(),
        }
        self
    }
}

fn compare_magnitudes(left: &str, right: &str) -> std::cmp::Ordering {
    left.len().cmp(&right.len()).then_with(|| left.cmp(right))
}

fn add_magnitudes(left: &str, right: &str) -> String {
    let mut carry = 0u8;
    let mut out = Vec::new();
    let mut left = left.bytes().rev();
    let mut right = right.bytes().rev();
    loop {
        let a = left.next().map(|byte| byte - b'0');
        let b = right.next().map(|byte| byte - b'0');
        if a.is_none() && b.is_none() && carry == 0 {
            break;
        }
        let sum = a.unwrap_or(0) + b.unwrap_or(0) + carry;
        out.push(b'0' + sum % 10);
        carry = sum / 10;
    }
    out.reverse();
    String::from_utf8(out).expect("decimal digits are UTF-8")
}

/// Subtract `right` from `left`; the caller proves `left >= right`.
fn subtract_magnitudes(left: &str, right: &str) -> String {
    let mut borrow = 0i8;
    let mut out = Vec::new();
    let mut right = right.bytes().rev();
    for byte in left.bytes().rev() {
        let mut digit = (byte - b'0') as i8 - borrow;
        let other = right.next().map(|byte| (byte - b'0') as i8).unwrap_or(0);
        if digit < other {
            digit += 10;
            borrow = 1;
        } else {
            borrow = 0;
        }
        out.push(b'0' + (digit - other) as u8);
    }
    while out.len() > 1 && out.last() == Some(&b'0') {
        out.pop();
    }
    out.reverse();
    String::from_utf8(out).expect("decimal digits are UTF-8")
}
