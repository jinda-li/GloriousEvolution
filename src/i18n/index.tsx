import { createContext, Fragment, useContext, useMemo } from "react";
import type { ReactNode } from "react";

import de from "./locales/de";
import en from "./locales/en";
import es from "./locales/es";
import fr from "./locales/fr";
import ja from "./locales/ja";
import ko from "./locales/ko";
import zhCN from "./locales/zh-CN";
import zhTW from "./locales/zh-TW";

export type MessageKey = keyof typeof en;
export type Messages = Record<MessageKey, string>;

export const LOCALES = [
  { id: "en", label: "English" },
  { id: "zh-CN", label: "简体中文" },
  { id: "zh-TW", label: "繁體中文" },
  { id: "ja", label: "日本語" },
  { id: "ko", label: "한국어" },
  { id: "es", label: "Español" },
  { id: "fr", label: "Français" },
  { id: "de", label: "Deutsch" },
] as const;

export type Locale = (typeof LOCALES)[number]["id"];

const MESSAGES: Record<Locale, Messages> = {
  en,
  "zh-CN": zhCN,
  "zh-TW": zhTW,
  ja,
  ko,
  es,
  fr,
  de,
};

/** Maps a BCP 47 tag (`zh-Hant-HK`, `en-US`, `de`) to a supported locale. */
export function matchLocale(tag: string): Locale | null {
  const lower = tag.toLowerCase().replace(/_/g, "-");
  if (lower.startsWith("zh")) {
    return lower.split("-").some((part) => ["tw", "hk", "mo", "hant"].includes(part)) ? "zh-TW" : "zh-CN";
  }
  const primary = lower.split("-")[0];
  return LOCALES.find((locale) => locale.id === primary)?.id ?? null;
}

/** `auto` follows the system (WebView) language; unsupported languages fall back to English. */
export function resolveLocale(setting: string): Locale {
  if (setting && setting !== "auto") {
    const explicit = matchLocale(setting);
    if (explicit) {
      return explicit;
    }
  }
  const preferred = navigator.languages?.length ? navigator.languages : [navigator.language];
  for (const tag of preferred) {
    const match = matchLocale(tag);
    if (match) {
      return match;
    }
  }
  return "en";
}

type Vars = Record<string, string | number>;

export type Translate = ((key: MessageKey, vars?: Vars) => string) & {
  /** Like `t`, but `{name}` placeholders may be filled with React nodes. */
  rich: (key: MessageKey, nodes: Record<string, ReactNode>) => ReactNode;
  locale: Locale;
};

export function createTranslator(locale: Locale): Translate {
  const messages = MESSAGES[locale];
  const template = (key: MessageKey) => messages[key] ?? en[key];

  const t = ((key: MessageKey, vars?: Vars) => {
    let text = template(key);
    for (const [name, value] of Object.entries(vars ?? {})) {
      text = text.split(`{${name}}`).join(String(value));
    }
    return text;
  }) as Translate;

  t.rich = (key, nodes) =>
    template(key)
      .split(/(\{\w+\})/)
      .map((part, index) => {
        const name = /^\{(\w+)\}$/.exec(part)?.[1];
        return <Fragment key={index}>{name && name in nodes ? nodes[name] : part}</Fragment>;
      });
  t.locale = locale;
  return t;
}

const I18nContext = createContext<Translate>(createTranslator("en"));

export function I18nProvider({ locale, children }: { locale: Locale; children: ReactNode }) {
  const t = useMemo(() => createTranslator(locale), [locale]);
  return <I18nContext.Provider value={t}>{children}</I18nContext.Provider>;
}

export function useT() {
  return useContext(I18nContext);
}
