use std::{collections::HashMap, net::Ipv4Addr, str::FromStr};

use log::debug;

use self::utils::*;

mod utils;

#[derive(Debug)]
struct Flags {
    qr: bool,     // 0表示查询请求，1表示查询应答
    opcode: u8,   // 4位, 0代表标准查询，1表示反向查询，2表示服务器状态请求
    aa: bool,     // 权威回答
    tc: bool,     // 截短
    rd: bool,     // 期望递归
    ra: bool,     // 可递归
    rsv: u8,      // 3位，保留位
    ret_code: u8, // 4位，响应码
}

impl Flags {
    fn parse(data: u16) -> Self {
        Self {
            qr: get_bit(data, 15),
            opcode: get_value(data, 11, 4) as u8,
            aa: get_bit(data, 10),
            tc: get_bit(data, 9),
            rd: get_bit(data, 8),
            ra: get_bit(data, 7),
            rsv: get_value(data, 4, 3) as u8,
            ret_code: get_value(data, 0, 4) as u8,
        }
    }
    fn to_be_bytes(&self) -> [u8; 2] {
        let mut data = 0;
        data = set_bit(data, 15, self.qr);
        data = set_value(data, 11, 4, self.opcode as u16);
        data = set_bit(data, 10, self.aa);
        data = set_bit(data, 9, self.tc);
        data = set_bit(data, 8, self.rd);
        data = set_bit(data, 7, self.ra);
        data = set_value(data, 4, 3, self.rsv as u16);
        data = set_value(data, 0, 4, self.ret_code as u16);
        data.to_be_bytes()
    }
}

#[derive(Debug)]
struct Header {
    // 共96位，即96/8=12字节
    identifier: u16, // 16位
    flags: Flags,
    question_count: u16,   // 问题数
    answer_count: u16,     // 答案数
    authority_count: u16,  // 授权信息数
    additional_count: u16, // 附加信息数
}

impl Header {
    fn parse(data: &[u8]) -> Self {
        Self {
            identifier: u16::from_be_bytes([data[0], data[1]]),
            flags: Flags::parse(u16::from_be_bytes([data[2], data[3]])),
            question_count: u16::from_be_bytes([data[4], data[5]]),
            answer_count: u16::from_be_bytes([data[6], data[7]]),
            authority_count: u16::from_be_bytes([data[8], data[9]]),
            additional_count: u16::from_be_bytes([data[10], data[11]]),
        }
    }
    fn reply(&self, answer_count: u16) -> Self {
        Self {
            identifier: self.identifier,
            flags: Flags {
                qr: true,
                opcode: 0,
                aa: true,
                tc: false,
                rd: false,
                ra: false,
                rsv: self.flags.rsv,
                ret_code: 0,
            },
            question_count: self.question_count,
            answer_count,
            authority_count: 0,
            additional_count: 0,
        }
    }
    fn stringify(&self) -> Vec<u8> {
        let mut data = Vec::new();
        data.extend(self.identifier.to_be_bytes());
        data.extend(self.flags.to_be_bytes());
        data.extend(self.question_count.to_be_bytes());
        data.extend(self.answer_count.to_be_bytes());
        data.extend(self.authority_count.to_be_bytes());
        data.extend(self.additional_count.to_be_bytes());
        data
    }
}

#[derive(Debug)]
struct Question {
    name: Vec<u8>, // 待查询的域名
    r#type: u16,   // 查询类型，1表示A记录，28表示AAAA IPv6，等等
    class: u16,    // 通常为 1 ，表示 TCP/IP 互联网地址；
}

impl Question {
    fn parse_name(data: &[u8]) -> String {
        let index = 0;
        let len = data[0];
        let mut names: Vec<String> = Vec::new();
        while len > 0 {
            let value = String::from_utf8_lossy(
                &data[((index + 1) as usize)..((index + 1 + len) as usize)],
            )
            .to_string();
            names.push(value);
        }
        return names.join(".");
    }
    fn get_name_len(data: &[u8]) -> usize {
        let mut index = 0;
        while data[index] > 0 {
            index += data[index] as usize + 1;
            if data[index] == 0 {
                return index + 1;
            }
        }
        return index;
    }
    fn parse(data: &[u8]) -> (Self, usize) {
        let name_len = Self::get_name_len(data);
        debug!("name len: {name_len}, {data:?}");
        let question = Self {
            name: data[..name_len].to_vec(),
            r#type: u16::from_be_bytes([data[name_len], data[name_len + 1]]),
            class: u16::from_be_bytes([data[name_len + 2], data[name_len + 3]]),
        };
        (question, name_len + 4)
    }
    fn parse_many(count: u16, data: &[u8]) -> (Vec<Self>, usize) {
        let mut last = 0;
        let mut questions = Vec::new();
        for _item in 0..count {
            let (question, len) = Self::parse(&data[last..]);
            questions.push(question);
            last += len;
        }
        (questions, last)
    }
    fn stringify(&self) -> Vec<u8> {
        let mut data = vec![];
        data.extend(&self.name);
        data.extend(self.r#type.to_be_bytes());
        data.extend(self.class.to_be_bytes());
        data
    }
}

impl Clone for Question {
    fn clone(&self) -> Self {
        Self {
            name: self.name.clone(),
            r#type: self.r#type,
            class: self.class,
        }
    }
}

#[derive(Debug)]
struct Answer {
    name: Vec<u8>, // 待查询的域名
    r#type: u16,   // 查询类型，1表示A记录，28表示AAAA IPv6，等等
    class: u16,    // 通常为 1 ，表示 TCP/IP 互联网地址；
    ttl: u32,
    resource_data_length: u16,
    resource_data: Vec<u8>,
}

impl Answer {
    fn parse(data: &[u8]) -> (Self, usize) {
        let (question, len) = Question::parse(data);
        let resource_data_length =
            u16::from_be_bytes(data[len + 4..len + 4 + 2].try_into().unwrap());
        let answer = Self {
            ttl: u32::from_be_bytes(data[len..len + 4].try_into().unwrap()),
            resource_data_length,
            resource_data: data[len + 4 + 2..].to_vec(),
            name: question.name,
            class: question.class,
            r#type: question.r#type,
        };
        (answer, len + 4 + 2 + (resource_data_length as usize))
    }
    fn parse_many(count: u16, data: &[u8]) -> (Vec<Self>, usize) {
        let mut last = 0;
        let mut answers = Vec::new();
        for _item in 0..count {
            let (question, len) = Self::parse(&data[last..]);
            answers.push(question);
            last += len;
        }
        (answers, last)
    }
    fn build(question: &Question, ttl: u32, value: &[u8]) -> Self {
        Self {
            name: question.name.clone(),
            r#type: question.r#type,
            class: question.class,
            ttl,
            resource_data_length: value.len() as u16,
            resource_data: value.to_vec(),
        }
    }

    fn stringify(&self) -> Vec<u8> {
        let mut data = vec![];
        data.extend(&self.name);
        data.extend(self.r#type.to_be_bytes());
        data.extend(self.class.to_be_bytes());
        data.extend(self.ttl.to_be_bytes());
        data.extend(self.resource_data_length.to_be_bytes());
        data.extend(&self.resource_data);
        data
    }
}

#[derive(Debug)]
struct Authority;

#[derive(Debug)]
struct Additional;

#[derive(Debug)]
pub struct Frame {
    header: Header,
    question: Vec<Question>,
    answer: Vec<Answer>,
    authority: Option<Authority>,
    additional: Option<Additional>,
}

impl Frame {
    pub fn parse(data: &[u8]) -> Self {
        let header = Header::parse(&data[..12]);
        let (questions, len) = Question::parse_many(header.question_count, &data[12..]);
        let (answers, len) = Answer::parse_many(header.answer_count, &data[12 + len..]);
        Self {
            header: header,
            question: questions,
            answer: answers,
            authority: None,
            additional: None,
        }
    }
    fn create_reply(&self, answers: Vec<Answer>) -> Self {
        Self {
            header: self.header.reply(answers.len() as u16),
            question: self.question.clone(),
            answer: answers,
            authority: None,
            additional: None,
        }
    }
    fn stringify(&self) -> Vec<u8> {
        let mut data = Vec::new();
        data.extend(self.header.stringify());
        let question = self
            .question
            .iter()
            .map(|question| question.stringify())
            .flatten();
        data.extend(question);
        let answer = self
            .answer
            .iter()
            .map(|answer| answer.stringify())
            .flatten();
        data.extend(answer);
        data
    }
}

fn transform_domain(domain: &str) -> Vec<u8> {
    domain
        .split(".")
        .map(|part| {
            let mut data = part.as_bytes().to_vec();
            data.insert(0, data.len() as u8);
            data
        })
        .flatten()
        .collect()
}

fn transform_ip(ip: &str) -> [u8; 4] {
    Ipv4Addr::from_str(ip).unwrap().octets()
}

pub fn handle(data: &[u8]) -> Vec<u8> {
    let mut origin_domains = HashMap::new();
    origin_domains.insert("www.baidu.com.", "127.0.0.1");

    let mut domains = HashMap::new();
    for (domain, ip) in origin_domains {
        domains.insert(transform_domain(domain), transform_ip(ip));
    }

    let frame = Frame::parse(data);
    debug!("frame: {frame:?}");

    let mut answers = Vec::new();
    for question in &frame.question {
        if let Some(value) = domains.get(&question.name) {
            debug!("find record");
            let answer = Answer::build(question, 10 * 60, value);
            answers.push(answer);
        }
    }

    // 生成应答报文
    let reply_frame = frame.create_reply(answers);
    reply_frame.stringify()
}
