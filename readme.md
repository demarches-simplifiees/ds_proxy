# DS Proxy

DS Proxy sert de proxy de chiffrement des fichiers entre l'application [démarches-simplifées](https://github.com/betagouv/demarches-simplifiees.fr/) et son backend de stockage.

Il se compose de 2 programmes:

 - le proxy
 - un générateur de hash

# Compilation

Le proxy est un applicatif [rust](rust-lang.org). La méthode préconisée pour installer le compilateur au sein de
la communauté est [rustup](https://rustup.rs/)

`Rustup`, le gestionnaire de versions de rust, va vous permettre d'installer la bonne version de `cargo`,
l'outil à tout faire qui permet notamment de piloter `rustc`, le compilateur. Ouf !

Concrètement, après avoir suivi l'installation de rustup, on peut compiler l'application en mode `debug` ou `release`
avec les commandes suivantes:

    $ cargo build
    $ cargo build --release

Vous pouvez également jouer les tests automatisés avec `cargo test` :

    $ cargo test

Afin de vous faciliter la vie, vous pouvez également regarder sur [AreWeIDEYet](https://areweideyet.com/) quels sont
les plugins les plus adaptés pour votre éditeur favori.

# Usage

Il faut tout d'abord générer un hash du mot de passe utilisé:

    $ create_hash_file hash.key

Il faut ensuite définir les variables d'environnement nécessaires:

    $ export UPSTREAM_URL="http://your.storage.backend";
    $ export DS_SALT="abcdefghabcdefghabcdefghabcdefgh";

Le sel DS_SALT doit faire 32 caractères.

On peut ensuite lancer le proxy:

    $ ./ds_proxy proxy localhost 8888 fichier_password

Le fichier password contenant le mot de passe.

# Utilisation via systemd

L'application est prévue pour tourner comme service avec systemd, qu'elle notifie de ses changements de status. Il faut donc déclarer son
utilisation dans un fichier avec a minima:

    # /etc/systemd/system/ds_proxy.service
    [Unit]
    Description=DS Proxy Service
    After=network.target

    [Service]
    WorkingDirectory=/home/ds_proxy
    ExecStart=/home/ds_proxy/ds_proxy proxy localhost 8888 password_file

    Environment=RUST_LOG="actix_web=info"
    Environment=UPSTREAM_URL="https://some.backend"
    Environment=DS_SALT="32 caracteres de sel"

    [Install]
    WantedBy=multi-user.target

Ensuite, on peut recharger ces modifications apportées à la configuration de systemd, lancer le service,
et vérifier qu'il a bien démarré:

    $ systemctl daemon-reload
    $ systemctl start ds_proxy
    $ systemctl status ds_proxys
