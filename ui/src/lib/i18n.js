import { addMessages, init } from '@sveltia/i18n'
import en from '../locales/en.json'
import de from '../locales/de.json'
import fr from '../locales/fr.json'
import es from '../locales/es.json'
import it from '../locales/it.json'
import pt from '../locales/pt.json'
import zh from '../locales/zh.json'
import ja from '../locales/ja.json'
import ko from '../locales/ko.json'
import sv from '../locales/sv.json'
import nl from '../locales/nl.json'

addMessages('en', en)
addMessages('de', de)
addMessages('fr', fr)
addMessages('es', es)
addMessages('it', it)
addMessages('pt', pt)
addMessages('zh', zh)
addMessages('ja', ja)
addMessages('ko', ko)
addMessages('sv', sv)
addMessages('nl', nl)

export function setupI18n(initialLocale) {
  init({ fallbackLocale: 'en', initialLocale: initialLocale ?? 'en' })
}
