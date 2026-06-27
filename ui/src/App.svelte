<script>
  import { Button }  from '$lib/components/ui/button/index.js'
  import { Input }   from '$lib/components/ui/input/index.js'
  import { Select }  from '$lib/components/ui/select/index.js'
  import { Card, CardContent } from '$lib/components/ui/card/index.js'
  import { Table, TableHeader, TableBody, TableRow, TableHead, TableCell } from '$lib/components/ui/table/index.js'
  import { setupI18n } from '$lib/i18n.js'
  import { _, locale } from '@sveltia/i18n'

  // ── App config ────────────────────────────────────────────────────────────
  let isDark = $state(document.documentElement.classList.contains('dark'))
  function toggleDark() {
    isDark = !isDark
    document.documentElement.classList.toggle('dark', isDark)
    localStorage.setItem('app-dark', String(isDark))
  }

  // ── Bibliography ──────────────────────────────────────────────────────────
  let bibDoi    = $state('')
  let bibStyle  = $state(localStorage.getItem('dragoman-style')  ?? 'apa')
  const initialLocale = localStorage.getItem('dragoman-locale') ?? (() => {
    const supported = ['en-US','de-DE','fr-FR','es-ES','it-IT','pt-BR','zh-CN','ja-JP','ko-KR']
    const lang = navigator.language
    return supported.find(v => v === lang)
        ?? supported.find(v => v.startsWith(lang.split('-')[0] + '-'))
        ?? 'en-US'
  })()
  setupI18n(initialLocale)
  let bibLocale = $state(initialLocale)
  let bibliography = $state((() => {
    try { return JSON.parse(localStorage.getItem('dragoman-bibliography') ?? '[]') }
    catch { return [] }
  })())
  let bibLoading   = $state(false)
  let bibError     = $state('')
  let copiedCiteId = $state(null)
  let dragId       = $state(null)
  let dropPosition = $state(null)  // { id, before: boolean }

  $effect(() => { localStorage.setItem('dragoman-bibliography', JSON.stringify(bibliography)) })
  $effect(() => { localStorage.setItem('dragoman-style',  bibStyle)  })
  $effect(() => { localStorage.setItem('dragoman-locale', bibLocale); locale.set(bibLocale) })

  // ── Export ────────────────────────────────────────────────────────────────
  let expFormat  = $state('citation')
  let expError   = $state('')
  let expLoading = $state(false)
  let expCopied  = $state(false)

  // ── Style picker ──────────────────────────────────────────────────────────
  let stylePickerOpen = $state(false)
  let styleSearch     = $state('')

  // ── Data ──────────────────────────────────────────────────────────────────
  const commonStyles = [
    { value: 'apa',                       label: 'American Psychological Association 7th edition' },
    { value: 'chicago-author-date',       label: 'Chicago Manual of Style 18th Edition (Author-Date)' },
    { value: 'harvard-cite-them-right',   label: 'Cite Them Right 12th Edition (Harvard)' },
    { value: 'ieee',                      label: 'IEEE' },
    { value: 'modern-language-association', label: 'MLA Handbook 9th Edition' },
    { value: 'vancouver',                 label: 'Vancouver' },
  ]

  const allCitationStyles = [
    { value: 'apa',                                              label: 'American Psychological Association 7th edition' },
    { value: 'alphanumeric',                                     label: 'Alphanumeric' },
    { value: 'american-anthropological-association',             label: 'American Anthropological Association' },
    { value: 'american-chemical-society',                        label: 'American Chemical Society' },
    { value: 'american-geophysical-union',                       label: 'American Geophysical Union' },
    { value: 'american-institute-of-aeronautics-and-astronautics', label: 'American Institute of Aeronautics and Astronautics' },
    { value: 'american-institute-of-physics',                    label: 'American Institute of Physics' },
    { value: 'american-medical-association',                     label: 'American Medical Association' },
    { value: 'american-meteorological-society',                  label: 'American Meteorological Society' },
    { value: 'american-physics-society',                         label: 'American Physical Society' },
    { value: 'american-physiological-society',                   label: 'American Physiological Society' },
    { value: 'american-political-science-association',           label: 'American Political Science Association' },
    { value: 'american-society-for-microbiology',                label: 'American Society for Microbiology' },
    { value: 'american-society-of-civil-engineers',              label: 'American Society of Civil Engineers' },
    { value: 'american-society-of-mechanical-engineers',         label: 'American Society of Mechanical Engineers' },
    { value: 'american-sociological-association',                label: 'American Sociological Association' },
    { value: 'angewandte-chemie',                                label: 'Angewandte Chemie International Edition' },
    { value: 'annual-reviews',                                   label: 'Annual Reviews' },
    { value: 'annual-reviews-author-date',                       label: 'Annual Reviews (Author-Date)' },
    { value: 'associacao-brasileira-de-normas-tecnicas',         label: 'Associação Brasileira de Normas Técnicas' },
    { value: 'association-for-computing-machinery',              label: 'Association for Computing Machinery' },
    { value: 'biomed-central',                                   label: 'BioMed Central' },
    { value: 'bmj',                                              label: 'BMJ' },
    { value: 'bristol-university-press',                         label: 'Bristol University Press' },
    { value: 'cell',                                             label: 'Cell' },
    { value: 'chicago-author-date',                              label: 'Chicago Manual of Style 18th Edition (Author-Date)' },
    { value: 'chicago-notes',                                    label: 'Chicago Manual of Style 18th Edition (Notes & Bibliography)' },
    { value: 'chicago-shortened-notes',                          label: 'Chicago Manual of Style 18th Edition (Shortened Notes)' },
    { value: 'copernicus',                                       label: 'Copernicus Publications' },
    { value: 'council-of-science-editors',                       label: 'Council of Science Editors (Citation-Sequence)' },
    { value: 'council-of-science-editors-author-date',           label: 'Council of Science Editors (Name-Year)' },
    { value: 'current-opinion',                                  label: 'Current Opinion' },
    { value: 'deutsche-gesellschaft-für-psychologie',            label: 'Deutsche Gesellschaft für Psychologie (Deutsch)' },
    { value: 'deutsche-sprache',                                 label: 'Deutsche Sprache (Deutsch)' },
    { value: 'elsevier-harvard',                                 label: 'Elsevier (Harvard)' },
    { value: 'elsevier-vancouver',                               label: 'Elsevier (Vancouver)' },
    { value: 'elsevier-with-titles',                             label: 'Elsevier (Numeric, With Titles)' },
    { value: 'frontiers',                                        label: 'Frontiers' },
    { value: 'future-medicine',                                  label: 'Future Medicine' },
    { value: 'future-science',                                   label: 'Future Science Group' },
    { value: 'gb-7714-2005-numeric',                             label: 'GB/T 7714-2005 (Numeric)' },
    { value: 'gb-7714-2015-author-date',                         label: 'GB/T 7714-2015 (Author-Date)' },
    { value: 'gb-7714-2015-note',                                label: 'GB/T 7714-2015 (Note)' },
    { value: 'gb-7714-2015-numeric',                             label: 'GB/T 7714-2015 (Numeric)' },
    { value: 'gost-r-705-2008-numeric',                          label: 'GOST R 7.0.5-2008 (Numeric)' },
    { value: 'harvard-cite-them-right',                          label: 'Cite Them Right 12th Edition (Harvard)' },
    { value: 'ieee',                                             label: 'IEEE' },
    { value: 'institute-of-physics-numeric',                     label: 'Institute of Physics (Numeric)' },
    { value: 'iso-690-author-date',                              label: 'ISO 690 (Author-Date)' },
    { value: 'iso-690-numeric',                                  label: 'ISO 690 (Numeric)' },
    { value: 'karger',                                           label: 'Karger' },
    { value: 'mary-ann-liebert-vancouver',                       label: 'Mary Ann Liebert (Vancouver)' },
    { value: 'modern-humanities-research-association',           label: 'MHRA Style Guide 4th Edition (Notes)' },
    { value: 'mla',                                              label: 'MLA Handbook 9th Edition' },
    { value: 'multidisciplinary-digital-publishing-institute',   label: 'Multidisciplinary Digital Publishing Institute' },
    { value: 'nature',                                           label: 'Nature' },
    { value: 'vancouver',                                        label: 'Vancouver' },
    { value: 'vancouver-superscript',                            label: 'NLM/Vancouver (Superscript)' },
    { value: 'pensoft',                                          label: 'Pensoft Journals' },
    { value: 'plos',                                             label: 'Public Library of Science' },
    { value: 'royal-society-of-chemistry',                       label: 'Royal Society of Chemistry' },
    { value: 'sage-vancouver',                                   label: 'SAGE (Vancouver)' },
    { value: 'sist02',                                           label: 'SIST02' },
    { value: 'spie',                                             label: 'SPIE' },
    { value: 'springer-basic',                                   label: 'Springer Basic (Numeric)' },
    { value: 'springer-basic-author-date',                       label: 'Springer Basic (Author-Date)' },
    { value: 'springer-fachzeitschriften-medizin-psychologie',   label: 'Springer Fachzeitschriften Medizin Psychologie (Deutsch)' },
    { value: 'springer-humanities-author-date',                  label: 'Springer Humanities (Author-Date)' },
    { value: 'springer-lecture-notes-in-computer-science',       label: 'Springer Lecture Notes in Computer Science' },
    { value: 'springer-mathphys',                                label: 'Springer MathPhys (Numeric)' },
    { value: 'springer-socpsych-author-date',                    label: 'Springer SocPsych (Author-Date)' },
    { value: 'springer-vancouver',                               label: 'Springer (Vancouver)' },
    { value: 'taylor-and-francis-chicago-author-date',           label: 'Taylor & Francis (Chicago Author-Date)' },
    { value: 'taylor-and-francis-national-library-of-medicine',  label: 'Taylor & Francis (NLM/Vancouver)' },
    { value: 'the-institution-of-engineering-and-technology',    label: 'Institution of Engineering and Technology' },
    { value: 'the-lancet',                                       label: 'The Lancet' },
    { value: 'thieme',                                           label: 'Thieme' },
    { value: 'trends',                                           label: 'Trends' },
    { value: 'turabian-author-date',                             label: 'Chicago Manual of Style 17th Edition (Author-Date)' },
    { value: 'turabian-fullnote-8',                              label: 'Chicago Manual of Style 17th Edition (Notes & Bibliography)' },
  ]

  const locales = [
    { value: 'en-US', label: 'English (US)' },
    { value: 'de-DE', label: 'Deutsch' },
    { value: 'fr-FR', label: 'Français' },
    { value: 'es-ES', label: 'Español' },
    { value: 'it-IT', label: 'Italiano' },
    { value: 'pt-BR', label: 'Português (Brasil)' },
    { value: 'zh-CN', label: '中文（简体）' },
    { value: 'ja-JP', label: '日本語' },
    { value: 'ko-KR', label: '한국어' },
  ]

  const expFormats = [
    { value: 'citation',     i18n: 'format.citation' },
    { value: 'commonmeta',   label: 'Commonmeta' },
    { value: 'bibtex',       label: 'BibTeX' },
    { value: 'csl',          label: 'CSL-JSON' },
    { value: 'crossref',     label: 'Crossref' },
    { value: 'crossref_xml', label: 'Crossref XML' },
    { value: 'datacite',     label: 'DataCite JSON' },
    { value: 'datacite_xml', label: 'DataCite XML' },
    { value: 'inveniordm',   label: 'InvenioRDM' },
    { value: 'ris',          label: 'RIS' },
    { value: 'schemaorg',    label: 'Schema.org' },
  ]

  // ── Helpers ───────────────────────────────────────────────────────────────
  function cleanDoi(raw) {
    return raw.trim().replace(/^https?:\/\/(?:dx\.)?doi\.org\//, '')
  }

  function stripHtml(html) {
    return html.replace(/<[^>]+>/g, '')
  }

  // ── Bibliography actions ──────────────────────────────────────────────────
  async function addToBib(e) {
    e.preventDefault()
    const id = cleanDoi(bibDoi)
    if (!id) return

    bibLoading = true
    bibError   = ''

    try {
      // Fetch commonmeta JSON once; stored so style changes need no external API calls.
      const dataResp = await fetch(`/${id}?format=commonmeta`)
      if (!dataResp.ok) {
        const text = await dataResp.text().catch(() => dataResp.statusText)
        throw new Error(`${dataResp.status}: ${text}`)
      }
      const cmData = await dataResp.text()
      const entryId = crypto.randomUUID()
      const entryUrl = (() => { try { return JSON.parse(cmData).url ?? null } catch { return null } })()

      const html = await formatItems([{ id: entryId, data: cmData }], bibStyle, bibLocale)
        .then(items => items[0]?.html ?? '')

      bibliography = [...bibliography, { id: entryId, doi: id, data: cmData, url: entryUrl, html }]
      bibDoi = ''
    } catch (err) {
      bibError = String(err)
    } finally {
      bibLoading = false
    }
  }

  async function formatItems(items, style, locale) {
    const resp = await fetch('/bibliography', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ items, style, locale }),
    })
    if (!resp.ok) {
      const text = await resp.text().catch(() => resp.statusText)
      throw new Error(`${resp.status}: ${text}`)
    }
    return (await resp.json()).items
  }

  function onBibStyleChange(e) {
    const val = e.target.value
    if (val === '__more__') {
      e.target.value = bibStyle
      stylePickerOpen = true
      styleSearch = ''
      return
    }
    bibStyle = val
    if (bibliography.length > 0) reformatBibliography(val, bibLocale)
  }

  function pickMoreStyle(style) {
    bibStyle = style.value
    stylePickerOpen = false
    styleSearch = ''
    if (bibliography.length > 0) reformatBibliography(style.value, bibLocale)
  }

  function onBibLocaleChange(e) {
    const locale = e.target.value
    bibLocale = locale
    if (bibliography.length > 0) reformatBibliography(bibStyle, locale)
  }

  async function reformatBibliography(style, locale) {
    bibLoading = true
    try {
      const formatted = await formatItems(
        bibliography.map(e => ({ id: e.id, data: e.data })),
        style,
        locale,
      )
      const byId = Object.fromEntries(formatted.map(i => [i.id, i.html]))
      bibliography = bibliography.map(e => ({ ...e, html: byId[e.id] ?? e.html }))
    } catch (err) {
      bibError = String(err)
    } finally {
      bibLoading = false
    }
  }

  function onDragStart(e, entry) {
    dragId = entry.id
    e.dataTransfer.effectAllowed = 'move'
  }

  function onDragOver(e, entry) {
    if (!dragId || dragId === entry.id) return
    e.preventDefault()
    const rect = e.currentTarget.getBoundingClientRect()
    dropPosition = { id: entry.id, before: e.clientY < rect.top + rect.height / 2 }
  }

  function onDragLeave(e) {
    if (!e.currentTarget.contains(e.relatedTarget)) dropPosition = null
  }

  function onDrop(e, entry) {
    e.preventDefault()
    if (!dragId || !dropPosition || dragId === entry.id) return
    const from = bibliography.findIndex(b => b.id === dragId)
    const moved = bibliography[from]
    const rest  = bibliography.filter(b => b.id !== dragId)
    const toIdx = rest.findIndex(b => b.id === dropPosition.id)
    rest.splice(dropPosition.before ? toIdx : toIdx + 1, 0, moved)
    bibliography = rest
    dragId = null
    dropPosition = null
  }

  function onDragEnd() {
    dragId = null
    dropPosition = null
  }

  function deleteEntry(id) {
    bibliography = bibliography.filter(e => e.id !== id)
  }

  function deleteBibliography() {
    bibliography = []
  }

  async function copyCitation(entry) {
    await navigator.clipboard.writeText(stripHtml(entry.html))
    copiedCiteId = entry.id
    setTimeout(() => { copiedCiteId = null }, 2000)
  }


  // ── Export actions ────────────────────────────────────────────────────────
  const expExtensions = { citation: 'txt', bibtex: 'bib', ris: 'ris', csl: 'json', commonmeta: 'json',
    crossref: 'json', crossref_xml: 'xml', datacite: 'json', datacite_xml: 'xml',
    inveniordm: 'json', schemaorg: 'json' }
  const jsonFormats = new Set(['csl', 'commonmeta', 'crossref', 'datacite', 'inveniordm', 'schemaorg'])

  async function fetchBibExport() {
    if (expFormat === 'citation') {
      return bibliography.map(e => stripHtml(e.html)).join('\n\n')
    }
    const results = await Promise.all(
      bibliography.map(async entry => {
        const resp = await fetch(`/${entry.doi}?format=${expFormat}`)
        if (!resp.ok) { const t = await resp.text().catch(() => resp.statusText); throw new Error(`${resp.status}: ${t}`) }
        return resp.text()
      })
    )
    return jsonFormats.has(expFormat)
      ? JSON.stringify(results.map(r => JSON.parse(r)), null, 2)
      : results.join('\n\n')
  }

  async function exportBibliography() {
    if (!bibliography.length) return
    expLoading = true
    expError = ''
    try {
      const combined = await fetchBibExport()
      const ext = expExtensions[expFormat] ?? 'txt'
      const url = URL.createObjectURL(new Blob([combined], { type: 'text/plain' }))
      Object.assign(document.createElement('a'), { href: url, download: `bibliography.${ext}` }).click()
      URL.revokeObjectURL(url)
    } catch (err) { expError = String(err) }
    finally { expLoading = false }
  }

  async function copyBibExport() {
    if (!bibliography.length) return
    expLoading = true
    expError = ''
    expCopied = false
    try {
      const combined = await fetchBibExport()
      await navigator.clipboard.writeText(combined)
      expCopied = true
      setTimeout(() => { expCopied = false }, 2000)
    } catch (err) { expError = String(err) }
    finally { expLoading = false }
  }
</script>

<div class="min-h-screen flex flex-col bg-background text-foreground antialiased">

  <!-- Header -->
  <header class="bg-[#f2f2f2] dark:bg-gray-800 shrink-0 border-b border-gray-200 dark:border-gray-700">
    <div class="max-w-4xl mx-auto px-6 h-14 flex items-center justify-between gap-4">
      <div class="flex items-center gap-2 min-w-0">
        <!-- Lightbulb icon matching python.commonmeta.org -->
        <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="currentColor" class="w-6 h-6 text-primary shrink-0">
          <path d="M12 .75a8.25 8.25 0 0 0-4.135 15.39c.686.398 1.115 1.143 1.115 1.942V18h5.25v-.008c0-.799.43-1.544 1.115-1.942A8.25 8.25 0 0 0 12 .75Z" />
          <path fill-rule="evenodd" d="M9.013 19.9a.75.75 0 0 1 .877-.597 11.319 11.319 0 0 0 4.22 0 .75.75 0 1 1 .28 1.473 12.819 12.819 0 0 1-4.78 0 .75.75 0 0 1-.597-.876ZM9 22.5a.75.75 0 0 1 .75-.75h4.5a.75.75 0 0 1 0 1.5h-4.5A.75.75 0 0 1 9 22.5Z" clip-rule="evenodd" />
        </svg>
        <span class="text-base font-bold tracking-tight shrink-0 text-gray-900 dark:text-white">Commonmeta</span>
        <span class="text-sm text-gray-600 dark:text-gray-400 truncate">{_('app.description')}</span>
      </div>
      <div class="flex items-center gap-3 shrink-0">
        <button
          type="button"
          onclick={toggleDark}
          aria-label="Toggle dark mode"
          class="text-gray-500 hover:text-gray-900 dark:text-gray-400 dark:hover:text-white transition-colors"
        >
          {#if isDark}
            <!-- Sun -->
            <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" class="w-5 h-5">
              <path stroke-linecap="round" stroke-linejoin="round" d="M12 3v2.25m6.364.386-1.591 1.591M21 12h-2.25m-.386 6.364-1.591-1.591M12 18.75V21m-4.773-4.227-1.591 1.591M5.25 12H3m4.227-4.773L5.636 5.636M15.75 12a3.75 3.75 0 1 1-7.5 0 3.75 3.75 0 0 1 7.5 0Z" />
            </svg>
          {:else}
            <!-- Moon -->
            <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" class="w-5 h-5">
              <path stroke-linecap="round" stroke-linejoin="round" d="M21.752 15.002A9.72 9.72 0 0 1 18 15.75c-5.385 0-9.75-4.365-9.75-9.75 0-1.33.266-2.597.748-3.752A9.753 9.753 0 0 0 3 11.25C3 16.635 7.365 21 12.75 21a9.753 9.753 0 0 0 9.002-5.998Z" />
            </svg>
          {/if}
        </button>
        <Select bind:value={bibLocale} onchange={onBibLocaleChange}
          class="h-7 w-36 text-xs py-0">
          {#each locales as l}
            <option value={l.value}>{l.label}</option>
          {/each}
        </Select>
      </div>
    </div>
  </header>

  <!-- Main -->
  <main class="flex-1 max-w-4xl w-full mx-auto px-6 py-10 space-y-12">

    <!-- ── Bibliography ─────────────────────────────────────────────────── -->
    <section>
      <h2 class="text-xl font-bold text-primary mb-4">{_('bibliography.title')}</h2>

      <!-- DOI input card -->
      <Card>
        <CardContent class="p-5">
          <form onsubmit={addToBib} class="flex gap-3">
            <Input
              type="text"
              bind:value={bibDoi}
              placeholder={_('bibliography.placeholder')}
              autocomplete="off"
              autocorrect="off"
              autocapitalize="none"
              spellcheck="false"
              class="font-mono flex-1"
            />
            <Button type="submit" disabled={bibLoading || !bibDoi.trim()}>
              {#if bibLoading}
                <span class="w-4 h-4 rounded-full border-2 border-primary-foreground/30 border-t-primary-foreground animate-spin"></span>
              {:else}
                {_('bibliography.add')}
              {/if}
            </Button>
          </form>

          {#if bibError}
            <div class="mt-3 px-4 py-3 bg-destructive/10 border border-destructive/20 rounded-md text-sm text-destructive" role="alert">
              {bibError}
            </div>
          {/if}
        </CardContent>
      </Card>

      <!-- Bibliography list -->
      {#if bibliography.length > 0}
        <!-- Style selector — prominent above list, like ZoteroBib -->
        <div class="mt-4 mb-2 relative">
          <Select value={bibStyle} onchange={onBibStyleChange} class="w-full">
            {#if !commonStyles.some(s => s.value === bibStyle)}
              <option value={bibStyle}>{allCitationStyles.find(s => s.value === bibStyle)?.label ?? bibStyle}</option>
              <option disabled>──────────────────────</option>
            {/if}
            {#each commonStyles as s}
              <option value={s.value}>{s.label}</option>
            {/each}
            <option disabled>──────────────────────</option>
            <option value="__more__">More styles…</option>
          </Select>

          {#if stylePickerOpen}
            <div class="fixed inset-0 z-40" role="presentation" onclick={() => { stylePickerOpen = false; styleSearch = '' }}></div>
            <div class="absolute top-full left-0 right-0 z-50 mt-1 bg-background border border-border rounded-md shadow-lg">
              <div class="p-2 border-b border-border">
                <!-- svelte-ignore a11y_autofocus -->
                <input
                  bind:value={styleSearch}
                  placeholder="Search styles…"
                  autofocus
                  class="w-full text-sm px-3 py-1.5 rounded border border-input bg-background outline-none focus:ring-1 focus:ring-ring"
                  onkeydown={e => { if (e.key === 'Escape') { stylePickerOpen = false; styleSearch = '' } }}
                />
              </div>
              <ul class="max-h-72 overflow-y-auto py-1">
                {#each allCitationStyles.filter(s => !styleSearch || s.label.toLowerCase().includes(styleSearch.toLowerCase())) as s}
                  <li>
                    <button
                      type="button"
                      class="w-full text-left px-3 py-1.5 text-sm hover:bg-muted {s.value === bibStyle ? 'font-semibold text-primary' : ''}"
                      onclick={() => pickMoreStyle(s)}
                    >{s.label}</button>
                  </li>
                {/each}
                {#if allCitationStyles.filter(s => !styleSearch || s.label.toLowerCase().includes(styleSearch.toLowerCase())).length === 0}
                  <li class="px-3 py-2 text-sm text-muted-foreground">No styles found</li>
                {/if}
              </ul>
            </div>
          {/if}
        </div>

        <div class="border border-border rounded-md overflow-hidden">
          <ol class="divide-y divide-border">
            {#each bibliography as entry, i (entry.id)}
              <li
                draggable="true"
                ondragstart={e => onDragStart(e, entry)}
                ondragover={e => onDragOver(e, entry)}
                ondragleave={onDragLeave}
                ondrop={e => onDrop(e, entry)}
                ondragend={onDragEnd}
                class="flex items-start gap-3 px-5 py-4 bg-background transition-opacity
                       {dragId === entry.id ? 'opacity-30' : ''}
                       {dropPosition?.id === entry.id && dropPosition.before  ? 'border-t-2 border-primary' : ''}
                       {dropPosition?.id === entry.id && !dropPosition.before ? 'border-b-2 border-primary' : ''}"
              >
                <!-- Grip handle -->
                <div class="shrink-0 pt-1 cursor-grab active:cursor-grabbing text-muted-foreground/30 hover:text-muted-foreground/60">
                  <svg viewBox="0 0 8 14" fill="currentColor" class="w-2 h-3.5">
                    <circle cx="2" cy="2"  r="1.5"/>
                    <circle cx="6" cy="2"  r="1.5"/>
                    <circle cx="2" cy="7"  r="1.5"/>
                    <circle cx="6" cy="7"  r="1.5"/>
                    <circle cx="2" cy="12" r="1.5"/>
                    <circle cx="6" cy="12" r="1.5"/>
                  </svg>
                </div>
                <span class="text-xs text-muted-foreground w-5 shrink-0 pt-0.5 text-right">{i + 1}.</span>
                <div class="flex-1 text-sm leading-relaxed
                            [&_a]:text-primary [&_a]:underline [&_a:hover]:no-underline">
                  {@html entry.html}
                </div>
                <!-- Per-entry icon buttons -->
                <div class="flex gap-0.5 shrink-0 pt-0.5">
                  {#if entry.url}
                    <a href={entry.url} target="_blank" rel="noreferrer"
                      class="inline-flex items-center justify-center h-7 w-7 rounded-md text-muted-foreground hover:text-foreground hover:bg-accent transition-colors"
                      aria-label="Open URL"
                    >
                      <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" class="w-4 h-4">
                        <path stroke-linecap="round" stroke-linejoin="round" d="M13.5 6H5.25A2.25 2.25 0 0 0 3 8.25v10.5A2.25 2.25 0 0 0 5.25 21h10.5A2.25 2.25 0 0 0 18 18.75V10.5m-10.5 6L21 3m0 0h-5.25M21 3v5.25" />
                      </svg>
                    </a>
                  {/if}
                  <Button type="button" variant="ghost" size="icon"
                    onclick={() => copyCitation(entry)}
                    aria-label={_('bibliography.copy_citation')}
                    class="h-7 w-7 text-muted-foreground hover:text-foreground"
                  >
                    {#if copiedCiteId === entry.id}
                      <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" class="w-4 h-4">
                        <path stroke-linecap="round" stroke-linejoin="round" d="m4.5 12.75 6 6 9-13.5" />
                      </svg>
                    {:else}
                      <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" class="w-4 h-4">
                        <path stroke-linecap="round" stroke-linejoin="round" d="M15.666 3.888A2.25 2.25 0 0 0 13.5 2.25h-3c-1.03 0-1.9.693-2.166 1.638m7.332 0c.055.194.084.4.084.612v0a.75.75 0 0 1-.75.75H9a.75.75 0 0 1-.75-.75v0c0-.212.03-.418.084-.612m7.332 0c.646.049 1.288.11 1.927.184 1.1.128 1.907 1.077 1.907 2.185V19.5a2.25 2.25 0 0 1-2.25 2.25H6.75A2.25 2.25 0 0 1 4.5 19.5V6.257c0-1.108.806-2.057 1.907-2.185a48.208 48.208 0 0 1 1.927-.184" />
                      </svg>
                    {/if}
                  </Button>
                  <Button type="button" variant="ghost" size="icon"
                    onclick={() => deleteEntry(entry.id)}
                    aria-label={_('bibliography.delete_entry')}
                    class="h-7 w-7 text-muted-foreground hover:text-destructive hover:bg-destructive/10"
                  >
                    <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" class="w-4 h-4">
                      <path stroke-linecap="round" stroke-linejoin="round" d="M6 18 18 6M6 6l12 12" />
                    </svg>
                  </Button>
                </div>
              </li>
            {/each}
          </ol>

          <!-- Bibliography-level actions -->
          <div class="border-t border-border bg-muted px-4 py-3 flex items-center gap-2 flex-wrap">
            <Select bind:value={expFormat} class="w-40 h-8 text-xs py-0">
              {#each expFormats as f}
                <option value={f.value}>{f.i18n ? _(f.i18n) : f.label}</option>
              {/each}
            </Select>
            <Button variant="default" onclick={exportBibliography} disabled={expLoading} class="h-8 gap-1.5 text-xs px-3">
              {#if expLoading}
                <span class="w-3.5 h-3.5 rounded-full border-2 border-primary-foreground/30 border-t-primary-foreground animate-spin"></span>
              {:else}
                <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" class="w-3.5 h-3.5 shrink-0">
                  <path stroke-linecap="round" stroke-linejoin="round" d="M3 16.5v2.25A2.25 2.25 0 0 0 5.25 21h13.5A2.25 2.25 0 0 0 21 18.75V16.5M16.5 12 12 16.5m0 0L7.5 12m4.5 4.5V3" />
                </svg>
              {/if}
              {_('bibliography.export')}
            </Button>
            <Button variant="outline" onclick={copyBibExport} disabled={expLoading} class="h-8 gap-1.5 text-xs px-3">
              {#if expCopied}
                <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" class="w-3.5 h-3.5 shrink-0">
                  <path stroke-linecap="round" stroke-linejoin="round" d="m4.5 12.75 6 6 9-13.5" />
                </svg>
                {_('bibliography.copied')}
              {:else}
                <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" class="w-3.5 h-3.5 shrink-0">
                  <path stroke-linecap="round" stroke-linejoin="round" d="M15.666 3.888A2.25 2.25 0 0 0 13.5 2.25h-3c-1.03 0-1.9.693-2.166 1.638m7.332 0c.055.194.084.4.084.612v0a.75.75 0 0 1-.75.75H9a.75.75 0 0 1-.75-.75v0c0-.212.03-.418.084-.612m7.332 0c.646.049 1.288.11 1.927.184 1.1.128 1.907 1.077 1.907 2.185V19.5a2.25 2.25 0 0 1-2.25 2.25H6.75A2.25 2.25 0 0 1 4.5 19.5V6.257c0-1.108.806-2.057 1.907-2.185a48.208 48.208 0 0 1 1.927-.184" />
                </svg>
                {_('bibliography.copy')}
              {/if}
            </Button>
            <Button
              variant="outline"
              onclick={deleteBibliography}
              class="h-8 gap-1.5 text-xs px-3 ml-auto text-destructive border-destructive/30 hover:bg-destructive/10 hover:text-destructive"
            >
              <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" class="w-3.5 h-3.5 shrink-0">
                <path stroke-linecap="round" stroke-linejoin="round" d="m14.74 9-.346 9m-4.788 0L9.26 9m9.968-3.21c.342.052.682.107 1.022.166m-1.022-.165L18.16 19.673a2.25 2.25 0 0 1-2.244 2.077H8.084a2.25 2.25 0 0 1-2.244-2.077L4.772 5.79m14.456 0a48.108 48.108 0 0 0-3.478-.397m-12 .562c.34-.059.68-.114 1.022-.165m0 0a48.11 48.11 0 0 1 3.478-.397m7.5 0v-.916c0-1.18-.91-2.164-2.09-2.201a51.964 51.964 0 0 0-3.32 0c-1.18.037-2.09 1.022-2.09 2.201v.916m7.5 0a48.667 48.667 0 0 0-7.5 0" />
              </svg>
              {_('bibliography.delete')}
            </Button>
            {#if expError}
              <p class="w-full text-xs text-destructive">{expError}</p>
            {/if}
          </div>
        </div>
      {/if}
    </section>

    <!-- ── Docs ─────────────────────────────────────────────────────────── -->
    <!-- Supported Metadata Formats table -->
    <div class="mb-8">
      <h2 class="text-xl font-bold text-primary border-b border-border pb-1 mb-4">{_('docs.formats.title')}</h2>
      <div class="rounded-md border border-border">
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead>Format</TableHead>
              <TableHead>Name</TableHead>
              <TableHead>Content Type</TableHead>
              <TableHead class="text-center">Read</TableHead>
              <TableHead class="text-center">Write</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            <TableRow>
              <TableCell>Commonmeta</TableCell>
              <TableCell class="font-mono text-xs">commonmeta</TableCell>
              <TableCell class="font-mono text-xs">application/vnd.commonmeta+json</TableCell>
              <TableCell class="text-center">✓</TableCell>
              <TableCell class="text-center">✓</TableCell>
            </TableRow>
            <TableRow>
              <TableCell><a href="http://en.wikipedia.org/wiki/BibTeX" class="text-primary hover:underline">BibTeX</a></TableCell>
              <TableCell class="font-mono text-xs">bibtex</TableCell>
              <TableCell class="font-mono text-xs">application/x-bibtex</TableCell>
              <TableCell class="text-center">✓</TableCell>
              <TableCell class="text-center">✓</TableCell>
            </TableRow>
            <TableRow>
              <TableCell><a href="https://citation-file-format.github.io/" class="text-primary hover:underline">Citation File Format (CFF)</a></TableCell>
              <TableCell class="font-mono text-xs">cff</TableCell>
              <TableCell class="font-mono text-xs">application/vnd.cff+yaml</TableCell>
              <TableCell class="text-center">✓</TableCell>
              <TableCell class="text-center text-muted-foreground">later</TableCell>
            </TableRow>
            <TableRow>
              <TableCell><a href="https://codemeta.github.io/" class="text-primary hover:underline">Codemeta</a></TableCell>
              <TableCell class="font-mono text-xs">codemeta</TableCell>
              <TableCell class="font-mono text-xs">application/vnd.codemeta.ld+json</TableCell>
              <TableCell class="text-center">✓</TableCell>
              <TableCell class="text-center text-muted-foreground">later</TableCell>
            </TableRow>
            <TableRow>
              <TableCell><a href="https://api.crossref.org" class="text-primary hover:underline">Crossref</a></TableCell>
              <TableCell class="font-mono text-xs">crossref</TableCell>
              <TableCell class="font-mono text-xs">application/vnd.crossref+json</TableCell>
              <TableCell class="text-center">✓</TableCell>
              <TableCell class="text-center">✓</TableCell>
            </TableRow>
            <TableRow>
              <TableCell><a href="https://www.crossref.org/schema/documentation/unixref1.1/unixref1.1.html" class="text-primary hover:underline">CrossRef XML</a></TableCell>
              <TableCell class="font-mono text-xs">crossref_xml</TableCell>
              <TableCell class="font-mono text-xs">application/vnd.crossref.unixref+xml</TableCell>
              <TableCell class="text-center">✓</TableCell>
              <TableCell class="text-center">✓</TableCell>
            </TableRow>
            <TableRow>
              <TableCell><a href="https://citationstyles.org/" class="text-primary hover:underline">CSL-JSON</a></TableCell>
              <TableCell class="font-mono text-xs">csl</TableCell>
              <TableCell class="font-mono text-xs">application/vnd.citationstyles.csl+json</TableCell>
              <TableCell class="text-center">✓</TableCell>
              <TableCell class="text-center">✓</TableCell>
            </TableRow>
            <TableRow>
              <TableCell><a href="https://api.datacite.org/" class="text-primary hover:underline">DataCite</a></TableCell>
              <TableCell class="font-mono text-xs">datacite</TableCell>
              <TableCell class="font-mono text-xs">application/vnd.datacite.datacite+json</TableCell>
              <TableCell class="text-center">✓</TableCell>
              <TableCell class="text-center">✓</TableCell>
            </TableRow>
            <TableRow>
              <TableCell><a href="https://api.datacite.org/" class="text-primary hover:underline">DataCite XML</a></TableCell>
              <TableCell class="font-mono text-xs">datacite_xml</TableCell>
              <TableCell class="font-mono text-xs">application/vnd.datacite.datacite+xml</TableCell>
              <TableCell class="text-center">✓</TableCell>
              <TableCell class="text-center">✓</TableCell>
            </TableRow>
            <TableRow>
              <TableCell><a href="https://citationstyles.org/" class="text-primary hover:underline">Formatted Citation</a></TableCell>
              <TableCell class="font-mono text-xs">citation</TableCell>
              <TableCell class="font-mono text-xs">text/x-bibliography</TableCell>
              <TableCell class="text-center text-muted-foreground">n/a</TableCell>
              <TableCell class="text-center">✓</TableCell>
            </TableRow>
            <TableRow>
              <TableCell><a href="https://inveniordm.docs.cern.ch/reference/metadata/" class="text-primary hover:underline">InvenioRDM</a></TableCell>
              <TableCell class="font-mono text-xs">inveniordm</TableCell>
              <TableCell class="font-mono text-xs">application/vnd.inveniordm.v1+json</TableCell>
              <TableCell class="text-center">✓</TableCell>
              <TableCell class="text-center">✓</TableCell>
            </TableRow>
            <TableRow>
              <TableCell><a href="https://www.jsonfeed.org/" class="text-primary hover:underline">JSON Feed</a></TableCell>
              <TableCell class="font-mono text-xs">jsonfeed</TableCell>
              <TableCell class="font-mono text-xs">application/feed+json</TableCell>
              <TableCell class="text-center">✓</TableCell>
              <TableCell class="text-center text-muted-foreground">later</TableCell>
            </TableRow>
            <TableRow>
              <TableCell><a href="https://www.openalex.org/" class="text-primary hover:underline">OpenAlex</a></TableCell>
              <TableCell class="font-mono text-xs">openalex</TableCell>
              <TableCell class="font-mono text-xs">n/a</TableCell>
              <TableCell class="text-center">✓</TableCell>
              <TableCell class="text-center text-muted-foreground">later</TableCell>
            </TableRow>
            <TableRow>
              <TableCell><a href="http://en.wikipedia.org/wiki/RIS_(file_format)" class="text-primary hover:underline">RIS</a></TableCell>
              <TableCell class="font-mono text-xs">ris</TableCell>
              <TableCell class="font-mono text-xs">application/x-research-info-systems</TableCell>
              <TableCell class="text-center">✓</TableCell>
              <TableCell class="text-center">✓</TableCell>
            </TableRow>
            <TableRow>
              <TableCell><a href="http://schema.org/" class="text-primary hover:underline">Schema.org (JSON-LD)</a></TableCell>
              <TableCell class="font-mono text-xs">schemaorg</TableCell>
              <TableCell class="font-mono text-xs">application/vnd.schemaorg.ld+json</TableCell>
              <TableCell class="text-center">✓</TableCell>
              <TableCell class="text-center">✓</TableCell>
            </TableRow>
          </TableBody>
        </Table>
      </div>
    </div>

    <div class="prose prose-sm prose-slate max-w-none
                [&_h2]:text-primary [&_h2]:border-b [&_h2]:border-border [&_h2]:pb-1
                [&_a]:text-primary [&_pre]:bg-muted [&_pre]:border [&_pre]:border-border
                [&_pre]:text-sm [&_pre]:text-gray-800 dark:[&_pre]:text-gray-200
                [&_pre]:my-3">

      <h2>{_('docs.usage.title')}</h2>
      <p>{_('docs.doi_resolution.intro')}</p>
      <pre><code>GET https://commonmeta.org/10.1371/journal.pcbi.1000204</code></pre>

      <h2>{_('docs.content_negotiation.title')}</h2>
      <p>{@html _('docs.content_negotiation.intro')}</p>
      <p>{_('docs.content_negotiation.bibtex_label')}</p>
      <pre><code>curl -H "Accept: application/x-bibtex" \
     https://commonmeta.org/10.1371/journal.pcbi.1000204</code></pre>

      <p>{@html _('docs.content_negotiation.format_param_label')}</p>
      <pre><code>curl https://commonmeta.org/10.1371/journal.pcbi.1000204?format=bibtex</code></pre>

      <p>{@html _('docs.content_negotiation.citation_style_label')}</p>
      <pre><code>curl -H "Accept: text/x-bibliography; style=vancouver; locale=de-DE" \
     https://commonmeta.org/10.1371/journal.pcbi.1000204</code></pre>
      <p>{_('docs.content_negotiation.query_params_label')}</p>
      <pre><code>curl "https://commonmeta.org/10.1371/journal.pcbi.1000204?format=citation&style=vancouver&locale=de-DE"</code></pre>

      <p>{@html _('docs.content_negotiation.multiple_types_label')}</p>
      <pre><code>curl -H "Accept: application/vnd.citationstyles.csl+json;q=0.9, application/x-bibtex" \
     https://commonmeta.org/10.1371/journal.pcbi.1000204</code></pre>
    </div>
  </main>

  <!-- Footer -->
  <footer class="py-4 text-xs text-gray-700 dark:text-gray-300">
    <div class="max-w-4xl mx-auto px-6 flex items-center justify-between">
      <span>
        Copyright &copy;2023&ndash;2026 Commonmeta.
        Built with <a href="https://github.com/front-matter/dragoman" target="_blank" rel="noreferrer"
           class="hover:text-gray-900 dark:hover:text-gray-100 transition-colors">dragoman</a>
        and <a href="https://github.com/front-matter/commonmeta-rs" target="_blank" rel="noreferrer"
           class="hover:text-gray-900 dark:hover:text-gray-100 transition-colors">commonmeta-rs</a>
        by <a href="https://front-matter.de" target="_blank" rel="noreferrer"
           class="hover:text-gray-900 dark:hover:text-gray-100 transition-colors">Front Matter</a>.
      </span>
      <span class="flex items-center gap-3">
        <a href="mailto:info@front-matter.de" aria-label="Email"
           class="hover:text-gray-900 dark:hover:text-gray-100 transition-colors">
          <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <rect width="20" height="16" x="2" y="4" rx="2"/><path d="m22 7-8.97 5.7a1.94 1.94 0 0 1-2.06 0L2 7"/>
          </svg>
        </a>
        <a href="https://hachyderm.io/@mfenner" target="_blank" rel="noreferrer" aria-label="Mastodon"
           class="hover:text-gray-900 dark:hover:text-gray-100 transition-colors">
          <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 16 16" fill="currentColor">
            <path d="M11.19 12.195c2.016-.24 3.77-1.475 3.99-2.603.348-1.778.32-4.339.32-4.339 0-3.47-2.286-4.488-2.286-4.488C12.062.238 10.083.017 8.027 0h-.05C5.92.017 3.942.238 2.79.765c0 0-2.285 1.017-2.285 4.488l-.002.662c-.004.64-.007 1.35.011 2.091.083 3.394.626 6.74 3.78 7.57 1.454.383 2.703.463 3.709.408 1.823-.1 2.847-.647 2.847-.647l-.06-1.317s-1.303.41-2.767.36c-1.45-.05-2.98-.156-3.215-1.928a4 4 0 0 1-.033-.496s1.424.346 3.228.428c1.103.05 2.137-.064 3.188-.189zm1.613-2.47H11.13v-4.08c0-.859-.364-1.295-1.091-1.295-.804 0-1.207.517-1.207 1.541v2.233H7.168V5.89c0-1.024-.403-1.541-1.207-1.541-.727 0-1.091.436-1.091 1.296v4.079H3.197V5.522q0-1.288.66-2.046c.456-.505 1.052-.764 1.793-.764.856 0 1.504.328 1.933.983L8 4.39l.417-.695c.429-.655 1.077-.983 1.934-.983.74 0 1.336.259 1.791.764q.662.757.661 2.046z"/>
          </svg>
        </a>
      </span>
    </div>
  </footer>

</div>
