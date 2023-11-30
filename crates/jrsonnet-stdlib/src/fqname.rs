use num_bigint::BigInt;
use num_traits::cast::ToPrimitive;
use jrsonnet_evaluator::{function::builtin, IStr};

struct Hash {
    data: BigInt,
}

impl Hash {
    fn new(initial: u64) -> Hash {
        Hash { data: BigInt::from(initial) }
    }

    fn update(&mut self, data: &str) {
        for char in data.chars() {
            let char_val = BigInt::from(char as u8);
            self.data = &char_val + (&self.data * BigInt::from(2).pow(6)) + (&self.data * BigInt::from(2).pow(16)) - &self.data;
        }
        let two_pow_64 = BigInt::from(2).pow(64);
        self.data %= &two_pow_64;
    }

    fn add(&mut self, val: u64) {
        self.data += BigInt::from(val);
    }

    fn get(&self) -> String {
        let base36 = "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ";
        let mut buf = vec!['0'; 14];
        let mut idx = buf.len() - 1;
        let mut remaining = self.data.clone();
        let thirty_six = BigInt::from(36);
        while remaining != BigInt::from(0) {
            let remainder = &remaining % &thirty_six;
            buf[idx] = base36.chars().nth(remainder.to_usize().unwrap()).unwrap();
            remaining /= &thirty_six;
            idx -= 1;
        }
        buf.iter().cloned().collect::<String>().trim_start_matches('0').to_string()
    }
}

#[builtin]
pub fn builtin_fqname(prefix : IStr, app: IStr, app_srv: IStr, be_name: IStr, nb_instance: usize) -> String {
    let mut result = Hash::new(1);
    result.update(app.as_str());
    result.update(app_srv.as_str());
    result.update(be_name.as_str());
    result.add(nb_instance as u64);
    result.update("salt");
    format!("{}{}", prefix, result.get())
}