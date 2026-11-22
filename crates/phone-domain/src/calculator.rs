//! Immediate-execution calculator with repeat-equals, memory and contextual percent.
//! User input is tokenized by button identity, never passed to a script evaluator.
#[derive(Clone, Debug)]
pub struct Calculator {
    entry: String,
    accumulator: Option<f64>,
    pending: Option<char>,
    repeated: Option<(char, f64)>,
    replace_entry: bool,
    memory: f64,
    error: bool,
}
impl Default for Calculator {
    fn default() -> Self {
        Self {
            entry: "0".into(),
            accumulator: None,
            pending: None,
            repeated: None,
            replace_entry: true,
            memory: 0.0,
            error: false,
        }
    }
}
impl Calculator {
    pub fn display(&self) -> String {
        self.entry.clone()
    }
    pub fn has_memory(&self) -> bool {
        self.memory != 0.0
    }
    pub fn press(&mut self, key: &str) -> String {
        match key {
            "0" | "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" | "." => self.digit(key),
            "+" => self.operator('+'),
            "-" | "−" => self.operator('-'),
            "*" | "×" => self.operator('*'),
            "/" | "÷" => self.operator('/'),
            "=" => self.equals(),
            "C" => {
                let memory = self.memory;
                *self = Self::default();
                self.memory = memory;
            }
            "CE" => {
                self.entry = "0".into();
                self.error = false;
                self.replace_entry = true;
            }
            "⌫" | "backspace" => {
                if !self.replace_entry && !self.error {
                    self.entry.pop();
                    if self.entry.is_empty() || self.entry == "-" {
                        self.entry = "0".into();
                    }
                }
            }
            "±" | "+/-" => {
                if !self.error && self.value() != 0.0 {
                    if self.entry.starts_with('-') {
                        self.entry.remove(0);
                    } else {
                        self.entry.insert(0, '-');
                    }
                }
            }
            "%" => {
                let value = self.value();
                self.assign(if matches!(self.pending, Some('+') | Some('-')) {
                    self.accumulator.unwrap_or(0.0) * value / 100.0
                } else {
                    value / 100.0
                });
            }
            "√" | "sqrt" => {
                let value = self.value();
                self.assign(value.sqrt());
            }
            "x²" | "square" => {
                let value = self.value();
                self.assign(value * value);
            }
            "1/x" => self.assign(1.0 / self.value()),
            "MC" => self.memory = 0.0,
            "MR" => {
                self.assign(self.memory);
                self.replace_entry = true;
            }
            "M+" => {
                if !self.error {
                    self.memory += self.value();
                    if !self.memory.is_finite() {
                        self.memory = 0.0;
                    }
                }
            }
            "M-" | "M−" => {
                if !self.error {
                    self.memory -= self.value();
                    if !self.memory.is_finite() {
                        self.memory = 0.0;
                    }
                }
            }
            _ => {}
        }
        self.display()
    }
    fn value(&self) -> f64 {
        self.entry.parse().unwrap_or(0.0)
    }
    fn digit(&mut self, key: &str) {
        if self.error {
            let memory = self.memory;
            *self = Self::default();
            self.memory = memory;
        }
        if self.replace_entry {
            self.entry = "0".into();
            self.replace_entry = false;
        }
        if key == "." {
            if !self.entry.contains('.') {
                self.entry.push('.');
            }
            return;
        }
        if self.entry.trim_start_matches('-').len() >= 16 {
            return;
        }
        if self.entry == "0" {
            self.entry = key.into();
        } else if self.entry == "-0" {
            self.entry = format!("-{key}");
        } else {
            self.entry.push_str(key);
        }
    }
    fn assign(&mut self, value: f64) {
        if !value.is_finite() {
            self.entry = "Error".into();
            self.error = true;
            self.pending = None;
            self.accumulator = None;
            self.repeated = None;
            self.replace_entry = true;
            return;
        }
        self.entry = if value == 0.0 {
            "0".into()
        } else if value.abs() >= 1e15 || value.abs() < 1e-9 {
            format!("{value:.9e}")
        } else {
            let fixed = format!("{value:.10}");
            fixed
                .trim_end_matches('0')
                .trim_end_matches('.')
                .to_string()
        };
        self.error = false;
    }
    fn calculate(left: f64, op: char, right: f64) -> f64 {
        match op {
            '+' => left + right,
            '-' => left - right,
            '*' => left * right,
            '/' => left / right,
            _ => right,
        }
    }
    fn operator(&mut self, op: char) {
        if self.error {
            return;
        }
        if let (Some(left), Some(pending)) = (self.accumulator, self.pending) {
            if !self.replace_entry {
                self.assign(Self::calculate(left, pending, self.value()));
                if self.error {
                    return;
                }
            }
        }
        self.accumulator = Some(self.value());
        self.pending = Some(op);
        self.repeated = None;
        self.replace_entry = true;
    }
    fn equals(&mut self) {
        if self.error {
            return;
        }
        if let Some(op) = self.pending.take() {
            let right = self.value();
            let left = self.accumulator.take().unwrap_or(right);
            self.assign(Self::calculate(left, op, right));
            if !self.error {
                self.repeated = Some((op, right));
            }
        } else if let Some((op, right)) = self.repeated {
            self.assign(Self::calculate(self.value(), op, right));
        }
        self.replace_entry = true;
    }
}
