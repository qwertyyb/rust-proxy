use std::{
    collections::{HashMap, HashSet},
    net::IpAddr,
    vec,
};

use log::{debug, error, warn};

use super::{utils::transform_domain, Answer, Question, RecordType};

type DnsRecord = Vec<u8>;

type ClassRecords = HashSet<DnsRecord>;

type DomainRecords = HashMap<RecordType, ClassRecords>;

pub struct Hosts {
    pub records: HashMap<Vec<u8>, DomainRecords>,
}

impl Hosts {
    fn load_system_hosts(&mut self) {
        const HOSTS_PATH: &str = "/etc/hosts";

        let hosts = std::fs::read_to_string(HOSTS_PATH);
        let mut results: Vec<(&str, &str)> = vec![];
        if let Ok(hosts) = hosts {
            hosts.lines().for_each(|line| {
                let line = line.trim();
                if line.is_empty() || line.starts_with("#") {
                    // 空行和注释忽略
                    return;
                }
                let parts: Vec<&str> = line.split_ascii_whitespace().collect();
                if parts.len() >= 2 {
                    results.push((parts[1], parts[0]))
                }
            });
            debug!("system hosts: {results:?}");
            self.insert_records(results);
        } else {
            error!("load system hosts failed: {hosts:?}")
        }
    }
    fn insert_records(&mut self, records: Vec<(&str, &str)>) {
        for (domain, ip) in records {
            let ip_addr: Result<IpAddr, _> = ip.parse();
            let domain = transform_domain(domain);
            let class_type;
            let record_value: Vec<u8>;
            if let Ok(ip_addr) = ip_addr {
                match ip_addr {
                    IpAddr::V4(ipv4) => {
                        class_type = RecordType::A;
                        record_value = ipv4.octets().to_vec();
                    }
                    IpAddr::V6(ipv6) => {
                        class_type = RecordType::AAAA;
                        record_value = ipv6.octets().to_vec();
                    }
                }
            } else {
                warn!("unrecognized value: {ip}");
                continue;
            }
            let record = self.records.get_mut(&domain);
            if let Some(record) = record {
                if let Some(resource) = record.get_mut(&class_type) {
                    resource.insert(record_value);
                } else {
                    let mut resource = HashSet::new();
                    resource.insert(record_value);
                    record.insert(class_type, resource);
                }
            } else {
                let mut value = HashMap::new();
                let mut resource = HashSet::new();
                resource.insert(record_value);
                value.insert(class_type, resource);
                self.records.insert(domain, value);
            }
        }
    }
    pub fn build() -> Self {
        let mut me = Self {
            records: HashMap::new(),
        };
        // 读取系统的hosts文件, unix/linux在/etc/hosts中
        me.load_system_hosts();
        me
    }
    pub fn search(&self, question: &Question) -> Option<Vec<Answer>> {
        let record = self.records.get(&question.name);
        if let Some(record) = record {
            if let Some(records) = record.get(&question.r#type) {
                return Some(
                    records
                        .iter()
                        .map(|record| Answer::build(question, 10 * 60, &record))
                        .collect(),
                );
            }
            None
        } else {
            None
        }
    }
}
