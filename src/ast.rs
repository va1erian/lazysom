use num_bigint::BigInt;

#[derive(Debug, Clone, PartialEq)]
pub struct ClassDef {
    pub name: String,
    pub super_class: Option<String>,
    pub instance_fields: Vec<String>,
    pub instance_methods: Vec<MethodDef>,
    pub class_fields: Vec<String>,
    pub class_methods: Vec<MethodDef>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MethodDef {
    pub signature: Signature,
    pub body: MethodBody,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Signature {
    Unary(String),
    Binary(String, String), // Name, Arg
    Keyword(Vec<(String, String)>), // (KeywordPart, Arg)
}

#[derive(Debug, Clone, PartialEq)]
pub enum MethodBody {
    Primitive,
    Block(Block),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Block {
    pub parameters: Vec<String>,
    pub locals: Vec<String>,
    pub body: Vec<Expression>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expression {
    Assignment(String, Box<Expression>),
    Message(Box<Expression>, Message),
    Variable(String),
    Literal(Literal),
    Block(Block),
    Return(Box<Expression>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Message {
    Unary(String),
    Binary(String, Box<Expression>),
    Keyword(Vec<(String, Box<Expression>)>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Literal {
    Integer(BigInt),
    Double(f64),
    String(String),
    Symbol(String),
    Array(Vec<Literal>),
}

impl Signature {
    pub fn selector(&self) -> String {
        match self {
            Signature::Unary(s) => s.clone(),
            Signature::Binary(s, _) => s.clone(),
            Signature::Keyword(parts) => {
                let mut sel = String::new();
                for (p, _) in parts {
                    sel.push_str(p);
                }
                sel
            }
        }
    }
}

impl Message {
    pub fn selector(&self) -> String {
        match self {
            Message::Unary(s) | Message::Binary(s, _) => s.clone(),
            Message::Keyword(parts) => {
                parts.iter().map(|(p, _)| p.as_str()).collect()
            }
        }
    }
}
