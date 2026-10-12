use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Deck {
    pub name: String,
    pub grid: Grid,
    pub pages: Vec<Page>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Grid {
    pub cols: u8,
    pub rows: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Page {
    pub name: String,
    pub buttons: Vec<Button>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Button {
    pub id: String,
    pub label: String,
    pub icon: Option<String>,
    #[serde(default)]
    pub background_image: Option<String>,
    #[serde(default)]
    pub custom_css: Option<String>,
    #[serde(default)]
    pub custom_html: Option<String>,
    pub on_press: Vec<Action>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Action {
    #[serde(rename = "action")]
    pub kind: String,
    #[serde(default)]
    pub keys: Vec<String>,
    pub text: Option<String>,
    pub url: Option<String>,
    pub command: Option<String>,
    pub delay: Option<f64>,
}