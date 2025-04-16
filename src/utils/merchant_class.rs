use lazy_static::lazy_static;
use std::collections::HashMap;
use strum::Display;

#[derive(Display, Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) enum MerchantType {
    #[strum(to_string = "食堂食物")]
    CanteenFood,
    #[strum(to_string = "食堂饮品")]
    CanteenDrink,
    #[strum(to_string = "超市")]
    Supermarket,
    #[strum(to_string = "浴室")]
    Bathhouse,
    #[strum(to_string = "其他")]
    Other,
    Unknown,
}

impl MerchantType {
    /// Converts a type string (like "食堂食物") to a MerchantType variant.
    fn from_type_str(s: &str) -> Self {
        match s {
            "食堂食物" => Self::CanteenFood,
            "食堂饮品" => Self::CanteenDrink,
            "超市" => Self::Supermarket,
            "浴室" => Self::Bathhouse,
            "其他" => Self::Other,
            _ => Self::Unknown, // Or handle error appropriately
        }
    }

    /// Gets the MerchantType based on the merchant name string using the global data.
    pub fn from_str(merchant_name: &str) -> Self {
        MERCHANT_DATA.get_type(merchant_name)
    }
}

#[derive(Debug)]
struct MerchantTypeData {
    data: HashMap<String, MerchantType>,
}

lazy_static! {
    static ref MERCHANT_DATA: MerchantTypeData = MerchantTypeData::new();
}

impl MerchantTypeData {
    fn new() -> Self {
        let mut data = HashMap::new();
        let config_str = include_str!("../../data/merchant-classification.yaml");
        // Ensure the YAML structure matches: Keys are type strings, values are lists of merchant strings.
        let config: HashMap<String, Vec<String>> =
            serde_yaml::from_str(config_str).expect("Failed to parse merchant classification YAML");

        for (type_str, merchants) in config {
            let merchant_type = MerchantType::from_type_str(&type_str);
            // Only proceed if the type is known (avoid inserting Unknown type directly from key)
            if merchant_type != MerchantType::Unknown {
                for merchant in merchants {
                    // Use entry API to avoid overwriting if a merchant appears under multiple types (last one wins here)
                    data.insert(merchant, merchant_type.clone());
                }
            } else {
                // Optionally log a warning for unknown types in YAML keys
                // eprintln!("Warning: Unknown merchant type key in YAML: {}", type_str);
            }
        }

        Self { data }
    }

    /// Looks up the MerchantType for a given merchant name.
    pub fn get_type(&self, merchant_name: &str) -> MerchantType {
        self.data
            .get(merchant_name)
            .cloned()
            .unwrap_or(MerchantType::Unknown) // Default to Unknown if not found
    }
}

// Optional: Add tests
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_merchant_type_lookup() {
        assert_eq!(MerchantType::from_str("炸吧"), MerchantType::CanteenFood);
        assert_eq!(MerchantType::from_str("时光水吧"), MerchantType::CanteenDrink);
        assert_eq!(
            MerchantType::from_str("鲜享优果水果店"),
            MerchantType::Supermarket
        );
        assert_eq!(
            MerchantType::from_str("东区浴室-和风"),
            MerchantType::Bathhouse
        );
        assert_eq!(MerchantType::from_str("自助补卡机"), MerchantType::Other);
        assert_eq!(
            MerchantType::from_str("NonExistentMerchant"),
            MerchantType::Unknown
        );
    }

    #[test]
    fn test_type_str_conversion() {
        assert_eq!(MerchantType::from_type_str("食堂食物"), MerchantType::CanteenFood);
        assert_eq!(MerchantType::from_type_str("超市"), MerchantType::Supermarket);
        assert_eq!(MerchantType::from_type_str("InvalidType"), MerchantType::Unknown);
    }
}
