# ALI - Agent Local Intelligence

ALI è un ambiente di lavoro locale con CLI e interfaccia desktop nativa per preparare brief da issue, esaminare localmente file di testo sensibili, mantenere memoria semantica cifrata e creare passaggi sintetici tra agenti. Supervisione automatica e failover degli agenti restano obiettivi della roadmap.

Lingue: [English](README.md) · [Italiano](README.it.md) · [Français](README.fr.md) · [Español](README.es.md)

## Confini di sicurezza attuali

- Testi dell’interfaccia, guida CLI e prompt sono caricati da `config/locales/`, `config/help/`, `config/prompts/`, `config/agent-prompts/` e `config/model-system/`. Le traduzioni mancanti causano un errore; gli errori tecnici non sono ancora tutti localizzati.
- Nessuna chiave API o token GitHub viene accettato da argomenti o variabili d’ambiente. La rifinitura del prompt accetta solo endpoint HTTP loopback. Gli embedding accettano solo host numerici loopback e non seguono redirect.
- I profili soggetti a restrizioni non possono inviare prompt a un modello. ALI non avvia ancora agenti di coding.
- I profili ristretti bloccano anche copia negli appunti e apertura del browser da `prepare` e dalla GUI. Sono protezioni del flusso selezionato, non autenticazione o confini del sistema operativo.
- Il profilo sviluppo è soggetto a restrizioni. ALI non legge né passa l’ambiente del processo agli agenti, che non vengono ancora avviati.
- La scansione e la revisione semantica dei documenti avvengono in locale. `privacy review` analizza con embedding loopback la copia dopo l’oscuramento lessicale e la scrive solo quando l’analisi termina; non la invia a un servizio remoto. Accetta solo i formati di testo UTF-8 configurati.
- Il rilevamento è euristico. Un esito pulito non dimostra che un file sia privo di dati sensibili. Dati legali, giudiziari, sanitari, finanziari e identificativi devono restare in locale.
- La memoria locale cifra testi e vettori con chiavi derivate tramite Argon2id e XChaCha20-Poly1305. La passphrase non viene salvata. Un processo dello stesso utente o un dispositivo compromesso può comunque accedere ai dati in chiaro durante l’uso.
- I passaggi sono limitati a un massimo configurato di 50 righe, ripuliti con regole configurabili e salvati con permessi restrittivi. Esaminali prima di trasferirli a un agente.

Consulta [sicurezza](docs/SECURITY.md), [modello delle minacce](docs/THREAT_MODEL.md), [ciclo dei dati](docs/DATA_LIFECYCLE.md) e [regole per i contributi](AGENTS.md).

## Compilazione

Richiede Rust 1.85 o successivo:

```sh
cargo build --release
```

`Cargo.lock` blocca le dipendenze. Non sono ancora disponibili installatori firmati o build di rilascio riproducibili.

## Brief da issue

```sh
cargo run -- prepare https://github.com/owner/repository/issues/123 --agent-id codex-cli --copy --open https://chatgpt.com/
cargo run -- --language it prepare https://github.com/owner/repository/issues/123 --output issue-123.prompt.md
```

Il recupero usa l’API pubblica di GitHub. L’accesso a repository privati non è implementato finché le credenziali non saranno integrate con il portachiavi del sistema. La rifinitura opzionale usa solo modelli locali:

```sh
cargo run -- prepare https://github.com/owner/repository/issues/123 \
  --model-endpoint http://127.0.0.1:11434 --model qwen2.5-coder
```

ALI non installa, autentica né avvia il server del modello. Gli endpoint fuori dal loopback vengono rifiutati.

## Memoria locale

Avvia Ollama e rendi disponibile il modello di embedding configurato. ALI non scarica modelli automaticamente.

```sh
cargo run -- memory init
cat note.txt | cargo run -- memory add --agent-id codex-cli
printf '%s' 'decisione precedente sul controllo accessi' | cargo run -- memory search --agent-id codex-cli
cargo run -- memory list
cargo run -- memory clear
```

La passphrase viene chiesta senza mostrarla nel terminale. Testo, tag agente facoltativo e vettori sono cifrati insieme. I tag sono manuali: ALI non raccoglie automaticamente l’attività degli agenti. La ricerca per agente include note condivise e quelle associate all’agente configurato. La richiesta di embedding raggiunge solo l’indirizzo loopback numerico di `config/runtime.toml`. Si può cambiare la directory con `memory --data-dir /percorso/privato`; ALI rifiuta symlink e permessi Unix troppo ampi.

La GUI nativa permette anche di inizializzare il vault, aggiungere e consultare note e cercare per agente configurato. Svuota i campi della passphrase e della query all’avvio dell’operazione e azzera i campi sensibili alla chiusura della finestra. La cancellazione del vault e la scelta di una directory personalizzata restano operazioni CLI.

## Revisione locale dei documenti

```sh
cargo run -- --language it privacy scan --input riservato.txt --output riservato.review.txt
cargo run -- --language it privacy analyze --input riservato.txt
cargo run -- --language it privacy review --input riservato.txt --output riservato.candidate.txt
```

I rilevatori configurati coprono alcuni pattern di email, token, chiavi private, codice fiscale italiano, SSN statunitense e carte di pagamento. `privacy analyze` confronta segmenti limitati con descrizioni semantiche multilingue tramite il servizio embedding loopback. Entrambe le operazioni mantengono il documento locale. Esamina l’intera copia e ogni segnalazione: la similarità non è una probabilità calibrata e non prova rilevamento, anonimizzazione, tutela del segreto professionale o autorizzazione all’invio. Nomi, fatti di causa, identificatori indiretti, immagini, grafia e formati sconosciuti possono non essere rilevati. I PDF e i documenti Office sono rifiutati finché non saranno disponibili parser sicuri e limitati.

`privacy review` oscura prima i pattern in memoria, analizza semanticamente la copia tramite loopback e la scrive solo quando l’analisi termina. Nessun comando trasferisce il documento a un servizio remoto.

## Passaggio di ibernazione

Fornisci lo schema JSON documentato tramite standard input. I campi sconosciuti e i valori multilinea vengono rifiutati:

```sh
cat handoff.json | cargo run -- handoff create
```

Campi obbligatori: `session_id`, `failed_agent`, `failure` (`crash`, `timeout`, `non_zero_exit`, `token_limit`, `cancelled` o `unknown`), `verified_progress` (fino a otto righe) e `next_action`. Il Markdown locale viene scansionato, conteggiato e creato senza sovrascrivere file esistenti. Il failover automatico non è ancora implementato.

## Configurazione e localizzazione

| Percorso | Contenuto parsato |
|---|---|
| `config/runtime.toml` | Lingue, profili, policy di rete, embedding locale, storage, formati documentali, KDF e limiti. |
| `config/agents.toml` | Chiavi degli agenti configurati e profili prompt. Gli agenti non vengono avviati. |
| `config/providers.toml` | API GitHub e percorso del servizio locale compatibile. |
| `config/sensitivity.toml` | Rilevatori locali di pattern sensibili. |
| `config/privacy-review.toml` | Riferimenti semantici multilingue, limiti di segmenti/batch/durata e soglia di similarità. |
| `config/help/{en,it,fr,es}.md` | Guida CLI localizzata mostrata con `--help`. |
| `config/locales/{en,it,fr,es}.toml` | Messaggi CLI e testi di sicurezza/consenso obbligatori. |
| `config/prompts/{en,it,fr,es}.md` | Template localizzati dei brief. |
| `config/agent-prompts/{en,it,fr,es}/{agente}.md` | Istruzioni localizzate specifiche dell’agente aggiunte ai brief generati. |
| `config/model-system/{en,it,fr,es}.md` | Istruzioni localizzate di rifinitura del modello. |
| `config/handoffs/{en,it,fr,es}.md` | Template localizzati dei passaggi. |

La configurazione viene caricata da `config/` rispetto alla directory corrente. `--config-dir` seleziona un’altra directory completa. Prima di iniziare, ALI valida i file e i template di tutte le lingue supportate.

## Ambito attuale e roadmap

I comandi CLI disponibili sono `prepare`, `memory`, `privacy scan`, `privacy analyze`, `privacy review`, `handoff create`, `agents list` e `gui`. La GUI nativa è disponibile; esecuzione/supervisione/failover automatici, profili autenticati, parsing PDF/Office e installatori restano milestone esplicite. Consulta la [roadmap](docs/ROADMAP.md), l’[architettura](docs/ARCHITECTURE.md), la guida [francese](README.fr.md) e la guida [spagnola](README.es.md).

## Catalogo degli adapter

Elenca i protocolli configurati con `ali adapters list`. Il catalogo viene letto da `config/adapters.toml`; le voci `planned` sono candidate future e non integrazioni eseguibili. Il client chat compatibile con OpenAI e gli embedding Ollama sono limitati al loopback. Consulta [interoperabilità](docs/INTEROPERABILITY.md), [job di consegna](docs/JOBS.md) e [roadmap](docs/ROADMAP.md).
