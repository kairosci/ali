# ALI - guide des commandes

ALI garde l’examen des documents, la mémoire et l’analyse des modèles sur cet appareil par défaut. Ce guide est chargé depuis le dossier de configuration sélectionné.

## Options globales

- `--config-dir CHEMIN` : choisit un dossier de configuration complet. Par défaut : `config` ; variable : `ALI_CONFIG_DIR`.
- `--language en|it|fr|es` : choisit la langue de l’interface et des prompts. Par défaut : `product.default_locale` dans `runtime.toml`.
- Sans commande, avec `--help`, `-h` ou `help` : affiche ce guide configuré.

## Commandes

- `ali prepare URL [--output CHEMIN] [--copy] [--open URL] [--model-endpoint URL_LOOPBACK --model NOM] [--profile PROFIL]` : récupère une issue GitHub publique par HTTPS et crée un brief local. Le raffinement est facultatif, local uniquement et désactivé pour les profils restreints. Ouvrir un site ne colle pas le brief.
- `ali memory init|add|search|list|clear [--data-dir CHEMIN]` : gère le coffre mémoire local chiffré. `add` et `search` lisent l’entrée standard ; `--agent-id ID` associe une note à un agent ou recherche les notes partagées et celles de cet agent configuré. Ollama doit déjà fonctionner à l’endpoint loopback configuré pour les embeddings.
- `ali privacy scan --input CHEMIN --output CHEMIN` : analyse localement un fichier texte UTF-8 configuré et crée une copie expurgée séparée.
- `ali privacy analyze --input CHEMIN` : compare des segments bornés aux descriptions sémantiques multilingues configurées via le service d’embeddings loopback. Les signaux ne décident pas d’une divulgation.
- `ali privacy review --input CHEMIN --output CHEMIN` : masque localement les motifs configurés, examine sémantiquement la copie avec les embeddings loopback et ne l’écrit qu’après réussite de l’analyse locale. Aucun transfert distant n’a lieu.
- `ali handoff create [--output CHEMIN]` : lit le schéma JSON validé sur l’entrée standard et crée une note locale analysée de 50 lignes au maximum.
- `ali agents list` : inspecte uniquement les chemins déclarés dans `config/agents.toml` ; aucun agent n’est lancé.
- `ali gui` : ouvre l’interface native.

## Frontière des données

Cette version ne comporte aucun compte et ne lance aucun agent. Récupérer une issue publique est une requête réseau GitHub explicite. L’analyse des documents, l’examen sémantique et les embeddings mémoire restent locaux ; les endpoints d’embedding doivent utiliser une adresse IP loopback. L’examen sémantique est une aide incomplète : il ne garantit ni détection, anonymat, secret professionnel, ni autorisation de divulguer. Gardez les données restreintes en local et vérifiez chaque résultat.

### adapters list

Affiche les protocoles de modèles configurés et leur état d’implémentation. Les éléments planifiés ne sont pas encore utilisables.
