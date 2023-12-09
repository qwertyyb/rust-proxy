pub fn get_bit(data: u16, pos: usize) -> bool {
    ((data & (1 << pos)) >> pos) == 1
}
pub fn get_value(data: u16, pos: usize, len: usize) -> u16 {
    (data << (16 - pos - len)) >> (16 - len)
}
pub fn set_bit(data: u16, pos: usize, value: bool) -> u16 {
    if value {
        data | (1 << pos)
    } else {
        data & !(1 << pos)
    }
}
pub fn set_value(data: u16, pos: usize, len: usize, value: u16) -> u16 {
    let mask: u16 = ((1 << len) - 1) << pos;
    let cleared_bits = data & !mask;
    let shifted_value = value << pos;
    cleared_bits | shifted_value
}

pub fn transform_domain(domain: &str) -> Vec<u8> {
    let mut result: Vec<u8> = domain
        .split(".")
        .map(|part| {
            let mut data = part.as_bytes().to_vec();
            data.insert(0, data.len() as u8);
            data
        })
        .flatten()
        .collect();
    if let Some(value) = result.last() {
        if *value != 0 {
            result.push(0)
        }
    }
    result
}

// pub fn transform_name(data: &[u8], offset: u16) {
//     let mut index = offset;
//     while data[index] > 0 {
//         let is_pointer = get_bit(data[index] as u16, 6) && get_bit(data[index] as u16, 7);
//         if is_pointer {
//             // 前两位是1，则后6位是offset
//             index = index + 1
//         } else {
//             index += data[index] as usize + 1;
//         }
//         if data[index] == 0 {
//             return index + 1;
//         }
//     }
//     return index;
// }

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_get_bit() {
        let cases: [(u16, usize, bool); 2] = [(0xf0f0, 4, true), (0xf0f0, 9, false)];
        for (data, pos, result) in cases {
            assert_eq!(
                get_bit(data, pos),
                result,
                "get_bit({data:02X}, {pos}) != {result}"
            )
        }
    }

    #[test]
    fn test_get_value() {
        // 0xf2: 11110010
        let cases: [(u16, usize, usize, u16); 4] = [
            (0xf2, 0, 1, 0),
            (0xf2, 1, 2, 1),
            (0xf2, 1, 3, 1),
            (0xf2, 1, 4, 9),
        ];
        for (data, pos, len, result) in cases {
            assert_eq!(
                get_value(data, pos, len),
                result,
                "get_value({data:02X}, {pos}, {len}) != {result}"
            );
        }
    }

    #[test]
    fn test_set_bit() {
        let cases: [(u16, usize, bool, u16); 4] = [
            (0x11, 0, false, 16),
            (0x11, 4, false, 1),
            (0x01, 0, false, 0),
            (0x01, 1, true, 3),
        ];
        for (data, pos, value, result) in cases {
            assert_eq!(
                set_bit(data, pos, value),
                result,
                "set_bit({data:02X}, {pos}, {value}) != {result}"
            );
        }
    }

    #[test]
    fn test_set_value() {
        let cases: [(u16, usize, usize, u16, u16); 3] = [
            (0b01010101, 1, 1, 0b1, 0b01010111),
            (0b01010101, 1, 2, 0b1, 0b01010011),
            (0b01010101, 2, 3, 0b10, 0b01001001),
        ];
        for (data, pos, len, value, result) in cases {
            assert_eq!(
                set_value(data, pos, len, value),
                result,
                "set_bit({data:08b}, {pos}, {len}, {value:08b}) != {result:08b}"
            );
        }
    }
}
