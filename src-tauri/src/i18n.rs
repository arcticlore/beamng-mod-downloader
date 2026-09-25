//! Локализация пользовательских сообщений бэкенда (RU/EN).
//!
//! Состояние языка хранится в процессе (одна Tauri-инстанция, один config),
//! поэтому вызывать `tf!` можно из любой глубины стека без прокидывания
//! параметра. Язык выставляется при старте из `Config::load()` и обновляется
//! через `set_app_settings`. По умолчанию (в т.ч. в юнит-тестах) — русский.

use std::sync::atomic::{AtomicU8, Ordering};

static LANG: AtomicU8 = AtomicU8::new(0);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lang {
    Ru,
    En,
}

impl Lang {
    pub fn parse(value: &str) -> Lang {
        match value {
            "en" | "EN" | "en-US" | "en_GB" | "en_US" => Lang::En,
            _ => Lang::Ru,
        }
    }

    /// Язык из переменных окружения (для новых конфигов): русская локаль → RU.
    pub fn detect_env() -> Lang {
        for var in ["LC_ALL", "LC_MESSAGES", "LANG", "LANGUAGE"] {
            if let Ok(v) = std::env::var(var) {
                let v = v.to_ascii_lowercase();
                if v.starts_with("ru") || v.starts_with("uk") || v.starts_with("be") {
                    return Lang::Ru;
                }
                if v.starts_with("en") {
                    return Lang::En;
                }
            }
        }
        Lang::En
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Lang::Ru => "ru",
            Lang::En => "en",
        }
    }
}

/// Выставляет язык процесса (из конфига при старте / при сохранении настроек).
pub fn set_lang(lang: Lang) {
    LANG.store(
        match lang {
            Lang::Ru => 0,
            Lang::En => 1,
        },
        Ordering::Relaxed,
    );
}

/// Текущий язык процесса; по умолчанию — русский (back-compat с v0.3.0).
pub fn lang() -> Lang {
    match LANG.load(Ordering::Relaxed) {
        0 => Lang::Ru,
        _ => Lang::En,
    }
}

/// Строка без аргументов: выбирает вариант текущего языка процесса.
pub fn t(ru: &str, en: &str) -> String {
    match lang() {
        Lang::Ru => ru.to_string(),
        Lang::En => en.to_string(),
    }
}

/// Строка с плейсхолдерами `{0}`, `{1}`, … — подставляет переданные аргументы.
pub fn tf(ru: &str, en: &str, args: &[impl ToString]) -> String {
    let mut out = match lang() {
        Lang::Ru => ru.to_string(),
        Lang::En => en.to_string(),
    };
    for (i, a) in args.iter().enumerate() {
        out = out.replace(&format!("{{{i}}}"), &a.to_string());
    }
    out
}

/// Русское множественное число: индекс формы 0/1/2 для чисел 1, 2–4, 5+.
pub fn ru_plural_index(n: u64) -> usize {
    let mod10 = n % 10;
    let mod100 = n % 100;
    if mod10 == 1 && mod100 != 11 {
        0
    } else if (2..=4).contains(&mod10) && !(12..=14).contains(&mod100) {
        1
    } else {
        2
    }
}

/// Строка с русским множественным числом: три формы RU (1, 2–4, 5+) и одна EN.
pub fn tfp(
    ru_one: &str,
    ru_few: &str,
    ru_many: &str,
    en: &str,
    n: u64,
    args: &[impl ToString],
) -> String {
    let ru = match ru_plural_index(n) {
        0 => ru_one,
        1 => ru_few,
        _ => ru_many,
    };
    tf(ru, en, args)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lang_parse() {
        assert_eq!(Lang::parse("ru"), Lang::Ru);
        assert_eq!(Lang::parse("ru-RU"), Lang::Ru);
        assert_eq!(Lang::parse("en"), Lang::En);
        assert_eq!(Lang::parse("en-US"), Lang::En);
        assert_eq!(Lang::parse(""), Lang::Ru);
    }

    #[test]
    fn default_is_russian() {
        assert_eq!(lang(), Lang::Ru);
        assert_eq!(t("русский", "english"), "русский");
        assert_eq!(
            tf("канал: {0}", "channel: {0}", &["x".to_string()]),
            "канал: x"
        );
    }

    #[test]
    fn set_lang_switches_messages() {
        set_lang(Lang::En);
        assert_eq!(lang(), Lang::En);
        assert_eq!(t("русский", "english"), "english");
        assert_eq!(
            tf("канал: {0}", "channel: {0}", &["x".to_string()]),
            "channel: x"
        );
        set_lang(Lang::Ru);
    }

    #[test]
    fn plural_rules_russian() {
        assert_eq!(ru_plural_index(1), 0);
        assert_eq!(ru_plural_index(21), 0);
        assert_eq!(ru_plural_index(2), 1);
        assert_eq!(ru_plural_index(5), 2);
        assert_eq!(ru_plural_index(11), 2);
        assert_eq!(ru_plural_index(12), 2);
        assert_eq!(ru_plural_index(111), 2);
    }
}
