# DS Proxy

DS Proxy sert de proxy de chiffrement des fichiers entre l'application et son backend.

Il se compose de 2 programmes:

 - le proxy
 - un générateur de hash

# Usage

Il faut tout d'abord générer un hash du mot de passe utilisé:

    $ create_hash_file hash.key

Il faut ensuite définir les variables d'environnement nécessaires:

    $ export UPSTREAM_URL="http://your.storage.backend";
    $ export DS_SALT="abcdefghabcdefghabcdefghabcdefgh";

Le sel DS_SALT doit faire 32 caractères.

On peut ensuite lancer le proxy:

    $ ./ds_proxy proxy localhost 8888 $(systemd-ask-password)

L'utilisation de systemd-ask-password permet de ne pas faire apparaitre le mot de passe dans le terminal

# Todo

## proxy

- gérer la config injectée
- gérer les erreurs et ajouter de la couverture de test
- meilleur logging (actix gère ça, à creuser. Lundi, nos tests ont généré 40Mo de logs en qques minutes…)
- gérer le header de fichier qui indique si c'est chiffré ou non. Pour ça c'est pas clair comment ajouter ça dans les encoders/decoders, les futures c'est pas hyper simple, si tu veux jeter un oeil…

## déploiement

 - dans les modifs ansible faudra donc ajouter la création d'un service systemd pour piloter le proxy et ne pas oublier de reporter les modifs qu'on a faites hier

## monitoring

 - ajout dans influxdb + un board grafana ?
   - à chaque upload/download, pour voir le volume de traitements?
   - ping d'une url /status, pour voir si le service est up ?