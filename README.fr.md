# ALI - Agent Local Intelligence

ALI est un espace de travail local avec CLI et interface de bureau native pour préparer des briefs à partir d’issues, examiner des fichiers texte sensibles, conserver une mémoire sémantique chiffrée et créer des relais compacts entre agents. La supervision automatique et le basculement entre agents restent dans la feuille de route.

Langues : [English](README.md) · [Italiano](README.it.md) · [Français](README.fr.md) · [Español](README.es.md)

## Limites de sécurité actuelles

- Les textes de l’interface, le guide CLI et les prompts viennent de `config/locales/`, `config/help/`, `config/prompts/` et `config/model-system/`. Une traduction manquante provoque une erreur ; les diagnostics techniques ne sont pas encore tous localisés.
- Aucune clé API ni aucun jeton GitHub n’est accepté dans les arguments ou les variables d’environnement. Le raffinement n’accepte qu’un endpoint HTTP loopback. Les embeddings exigent une adresse IP loopback et ne suivent aucune redirection.
- Les profils à accès restreint ne peuvent pas envoyer de prompt à un modèle. ALI ne lance pas encore d’agents de code.
- Les profils restreints bloquent aussi la copie dans le presse-papiers et l’ouverture du navigateur depuis `prepare` et l’interface. Ce sont des garde-fous de flux choisis, pas une authentification ni une frontière de sécurité du système.
- Le profil développement est restreint. ALI ne lit ni ne transmet l’environnement du processus aux agents, qui ne sont pas encore lancés.
- L’analyse et l’examen sémantique des documents restent locaux. `privacy review` examine par embeddings loopback la copie après masquage lexical et ne l’écrit qu’après réussite ; aucun service distant ne reçoit le document. Seuls les formats texte UTF-8 configurés sont acceptés.
- La détection est heuristique. Un résultat propre ne prouve pas l’absence de données sensibles. Les données juridiques, judiciaires, médicales, financières et d’identité doivent rester locales.
- La mémoire locale chiffre les textes et vecteurs avec des clés dérivées par Argon2id et XChaCha20-Poly1305. La phrase secrète n’est pas enregistrée. Un processus du même utilisateur ou un appareil compromis peut accéder aux données en clair pendant leur utilisation.
- Les relais sont limités à 50 lignes au maximum, nettoyés selon les règles configurées et enregistrés avec des permissions strictes. Examinez-les avant transmission à un agent.

Consultez la [sécurité](docs/SECURITY.md), le [modèle de menace](docs/THREAT_MODEL.md), le [cycle de vie des données](docs/DATA_LIFECYCLE.md) et les [règles de contribution](AGENTS.md).

## Compilation

Rust 1.85 ou supérieur :

```sh
cargo build --release
```

`Cargo.lock` verrouille les dépendances. Les installateurs signés et les builds de publication reproductibles ne sont pas encore fournis.

## Brief d’issue

```sh
cargo run -- prepare https://github.com/owner/repository/issues/123 --copy --open https://chatgpt.com/
cargo run -- --language fr prepare https://github.com/owner/repository/issues/123 --output issue-123.prompt.md
```

La récupération utilise l’API publique de GitHub. L’authentification aux dépôts privés n’est pas implémentée tant que les identifiants ne sont pas intégrés au trousseau du système. Le raffinement facultatif reste local :

```sh
cargo run -- prepare https://github.com/owner/repository/issues/123 \
  --model-endpoint http://127.0.0.1:11434 --model qwen2.5-coder
```

ALI n’installe, n’authentifie et ne lance pas le serveur de modèle. Les endpoints hors loopback sont refusés.

## Mémoire locale

Démarrez Ollama et rendez disponible le modèle d’embeddings configuré. ALI ne télécharge aucun modèle automatiquement.

```sh
cargo run -- memory init
cat notes.txt | cargo run -- memory add --agent-id codex-cli
printf '%s' 'ancienne décision sur le contrôle d’accès' | cargo run -- memory search --agent-id codex-cli
cargo run -- memory list
cargo run -- memory clear
```

La phrase secrète est demandée sans affichage dans le terminal. Texte, tag d’agent facultatif et vecteurs sont chiffrés ensemble. Les tags sont manuels : ALI ne collecte pas l’activité des agents. La recherche par agent inclut les notes partagées et celles de cet agent configuré. L’appel d’embedding cible uniquement l’adresse IP loopback de `config/runtime.toml`. `memory --data-dir /chemin/privé` permet de changer de dossier ; les liens symboliques et permissions Unix trop larges sont refusés.

L’interface native permet aussi d’initialiser le coffre, d’ajouter et consulter des notes et de rechercher par agent configuré. Elle efface les champs de phrase secrète et de requête au lancement de l’opération, puis met à zéro les champs sensibles à la fermeture. L’effacement du coffre et le choix d’un dossier personnalisé restent disponibles dans le CLI.

## Examen local d’un document

```sh
cargo run -- --language fr privacy scan --input confidentiel.txt --output confidentiel.review.txt
cargo run -- --language fr privacy analyze --input confidentiel.txt
cargo run -- --language fr privacy review --input confidentiel.txt --output confidentiel.candidate.txt
```

Les détecteurs configurés couvrent certains formats d’e-mail, jetons, clés privées, code fiscal italien, SSN américain et cartes de paiement. `privacy analyze` compare des segments bornés aux descriptions sémantiques multilingues configurées via le service d’embeddings loopback. Les deux opérations gardent le document local. Examinez toute la copie et chaque signalement : la similarité n’est pas une probabilité calibrée et ne prouve ni détection, ni anonymisation, ni secret professionnel, ni autorisation d’envoi. Les noms, faits de procédure, identifiants indirects, images, écritures manuscrites et formats inconnus peuvent ne pas être détectés. Les PDF et documents Office sont refusés jusqu’à l’intégration de parseurs sûrs et bornés.

`privacy review` masque d’abord les motifs en mémoire, analyse sémantiquement la copie via loopback et ne l’écrit qu’après réussite de l’analyse. Aucune commande ne transmet le document à un service distant.

## Relais d’hibernation

Fournissez le schéma JSON documenté sur l’entrée standard. Les champs inconnus et valeurs multiligne sont refusés :

```sh
cat handoff.json | cargo run -- handoff create
```

Champs obligatoires : `session_id`, `failed_agent`, `failure` (`crash`, `timeout`, `non_zero_exit`, `token_limit`, `cancelled` ou `unknown`), `verified_progress` (huit lignes maximum) et `next_action`. Le Markdown local est analysé, compté et créé sans écraser un fichier existant. Le basculement automatique entre agents n’est pas encore implémenté.

## Configuration et langues

| Chemin | Contenu analysé |
|---|---|
| `config/runtime.toml` | Langues, profils, politique réseau, embeddings locaux, stockage, formats, KDF et limites. |
| `config/providers.toml` | API GitHub et chemin du service local compatible. |
| `config/sensitivity.toml` | Détecteurs locaux de motifs sensibles. |
| `config/privacy-review.toml` | Références sémantiques multilingues, limites des segments/batches/de durée et seuil de similarité. |
| `config/help/{en,it,fr,es}.md` | Guide CLI localisé affiché avec `--help`. |
| `config/locales/{en,it,fr,es}.toml` | Messages CLI obligatoires et textes de sécurité/consentement. |
| `config/prompts/{en,it,fr,es}.md` | Modèles de briefs localisés. |
| `config/model-system/{en,it,fr,es}.md` | Instructions localisées de raffinement du modèle. |
| `config/handoffs/{en,it,fr,es}.md` | Modèles de relais localisés. |

La configuration est lue dans `config/` par rapport au répertoire courant. `--config-dir` sélectionne un autre dossier complet. ALI valide les fichiers et modèles de toutes les langues avant de commencer.

## Portée actuelle et feuille de route

Les commandes CLI disponibles sont `prepare`, `memory`, `privacy scan`, `privacy analyze`, `privacy review`, `handoff create`, `agents list` et `gui`. L’interface native est disponible ; l’exécution/supervision/basculement automatiques, les profils authentifiés, l’analyse PDF/Office et les installateurs restent des jalons explicites. Consultez la [feuille de route](docs/ROADMAP.md), l’[architecture](docs/ARCHITECTURE.md), le guide [italien](README.it.md) et le guide [espagnol](README.es.md).

## Catalogue des adaptateurs

Affiche les protocoles configurés avec `ali adapters list`. Le catalogue est lu depuis `config/adapters.toml`; les éléments `planned` sont des pistes futures, pas des intégrations utilisables. Le client de chat compatible OpenAI et les embeddings Ollama sont limités à la boucle locale. Consultez [interopérabilité](docs/INTEROPERABILITY.md), [travaux de livraison](docs/JOBS.md) et [feuille de route](docs/ROADMAP.md).
