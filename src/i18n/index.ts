import i18n from "i18next";
import { initReactI18next } from "react-i18next";
import commonEn from "./locales/en/common.json";
import libraryEn from "./locales/en/library.json";
import metadataEn from "./locales/en/metadata.json";
import searchEn from "./locales/en/search.json";
import commonFr from "./locales/fr/common.json";
import libraryFr from "./locales/fr/library.json";
import metadataFr from "./locales/fr/metadata.json";
import searchFr from "./locales/fr/search.json";

i18n.use(initReactI18next).init({
  resources: {
    en: {
      common: commonEn,
      library: libraryEn,
      metadata: metadataEn,
      search: searchEn,
    },
    fr: {
      common: commonFr,
      library: libraryFr,
      metadata: metadataFr,
      search: searchFr,
    },
  },
  defaultNS: "common",
  lng: "en",
  fallbackLng: "en",
  interpolation: { escapeValue: false },
});

export default i18n;
