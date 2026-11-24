# ALI - guida ai comandi

ALI mantiene in locale la revisione dei documenti, la memoria e l’analisi dei modelli. Questa guida viene caricata dalla directory di configurazione selezionata.

## Opzioni globali

- `--config-dir PERCORSO`: seleziona una directory di configurazione completa. Predefinita: `config`; variabile: `ALI_CONFIG_DIR`.
- `--language en|it|fr|es`: seleziona lingua dell’interfaccia e dei prompt. Predefinita: `product.default_locale` in `runtime.toml`.
- Nessun comando, `--help`, `-h` o `help`: mostra questa guida configurata.

## Comandi

- `ali prepare URL [--output PERCORSO] [--copy] [--open URL] [--model-endpoint URL_LOOPBACK --model NOME] [--profile PROFILO] [--agent-id ID]`: recupera via HTTPS una issue pubblica GitHub e crea un brief locale. La rifinitura è facoltativa, solo locale e disabilitata nei profili soggetti a restrizioni. Aprire un sito non incolla il brief.
- `ali memory init|add|search|list|clear [--data-dir PERCORSO]`: gestisce il vault cifrato locale. `add` e `search` leggono da standard input; `--agent-id ID` associa una nota a un agente o cerca le note condivise e quelle associate a quell’agente configurato. Ollama deve essere già attivo all’endpoint loopback configurato per le operazioni con embedding.
- `ali privacy scan --input PERCORSO --output PERCORSO`: analizza localmente un file di testo UTF-8 configurato e crea una copia redatta separata.
- `ali privacy analyze --input PERCORSO`: confronta segmenti limitati del testo con descrizioni semantiche multilingue configurate tramite il servizio embedding loopback. Le segnalazioni non autorizzano l’invio.
- `ali privacy review --input PERCORSO --output PERCORSO`: oscura localmente i pattern configurati, analizza semanticamente la copia con embedding loopback e la scrive solo se l’analisi locale termina. Non trasferisce mai il documento a un servizio remoto.
- `ali handoff create [--output PERCORSO]`: legge da standard input lo schema JSON validato e crea una nota locale scansionata di massimo 50 righe.
- `ali agents list`: controlla solo i percorsi dei comandi dichiarati in `config/agents.toml`; non avvia agenti.
- `ali web`: apre l’interfaccia browser locale su loopback. Prepara prompt da issue pubbliche GitHub; non accetta caricamenti di documenti.
- `ali gui`: apre l’interfaccia desktop nativa.

## Confine dei dati

Questa versione non ha account e non avvia agenti. Il recupero di issue pubbliche è una richiesta di rete GitHub esplicita. Scansione documenti, analisi semantica ed embedding della memoria sono locali; gli endpoint embedding devono usare un indirizzo numerico loopback. L’analisi semantica è un aiuto incompleto: non garantisce rilevamento, anonimato, segreto professionale o autorizzazione alla divulgazione. Mantieni in locale i dati soggetti a restrizioni e controlla ogni risultato.

### adapters list

Mostra i protocolli modello configurati e il loro stato di implementazione. Le voci pianificate non sono ancora utilizzabili.
