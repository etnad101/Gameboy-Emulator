#[cfg(test)]
use serde::Deserialize;

#[cfg(test)]
#[derive(Deserialize, Debug)]
pub struct TestJson {
    pub a: u8,
    pub b: u8,
    pub c: u8,
    pub d: u8,
    pub e: u8,
    pub f: u8,
    pub h: u8,
    pub l: u8,
    pub pc: u16,
    pub sp: u16,
    pub ram: Vec<Vec<u16>>,
}

#[cfg(test)]
#[derive(Deserialize, Debug)]
pub struct TestCase {
    pub name: String,
    pub initial: TestJson,
    #[serde(rename = "final")]
    pub final_name: TestJson,
}
