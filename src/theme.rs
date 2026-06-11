use gpui::{rgb, transparent_black, Hsla};

pub fn chrome() -> Hsla {
    rgb(0xfffefa).into()
}

pub fn paper() -> Hsla {
    rgb(0xfffdf9).into()
}

pub fn sidebar() -> Hsla {
    rgb(0xf8f5ee).into()
}

pub fn sidebar_control() -> Hsla {
    rgb(0xefebe1).into()
}

pub fn selected_note() -> Hsla {
    rgb(0xeaf3ed).into()
}

pub fn control() -> Hsla {
    rgb(0xf1eee6).into()
}

pub fn line() -> Hsla {
    rgb(0xeee9df).into()
}

pub fn divider() -> Hsla {
    rgb(0xb4b8ae).into()
}

pub fn text() -> Hsla {
    rgb(0x30302b).into()
}

pub fn text_muted() -> Hsla {
    rgb(0x7d7d72).into()
}

pub fn text_faint() -> Hsla {
    rgb(0xb8b5aa).into()
}

pub fn accent() -> Hsla {
    rgb(0x7ca184).into()
}

pub fn accent_dark() -> Hsla {
    rgb(0x557461).into()
}

pub fn danger() -> Hsla {
    rgb(0xb15b4f).into()
}

pub fn transparent() -> Hsla {
    transparent_black()
}
