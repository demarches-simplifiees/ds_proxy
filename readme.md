# DS Proxy

## Contexte

DS Proxy sert de proxy de chiffrement des fichiers entre l'application [démarches-simplifées](https://github.com/betagouv/demarches-simplifiees.fr/) et son backend de stockage.

Il se compose de 2 programmes:

 - le proxy
 - un générateur de hash

## Compilation

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

## Usage

Il faut tout d'abord générer un hash du mot de passe utilisé:

    $ create_hash_file hash.key

Il faut ensuite définir les variables d'environnement nécessaires:

    $ export UPSTREAM_URL="http://your.storage.backend";
    $ export DS_SALT="abcdefghabcdefghabcdefghabcdefgh";

Le sel DS_SALT doit faire 32 caractères.

On peut ensuite lancer le proxy:

    $ ./ds_proxy proxy localhost 8888 fichier_password

Le fichier password contenant le mot de passe. 

## Comment contribuer ?

**ds_proxy** est un [logiciel libre](https://fr.wikipedia.org/wiki/Logiciel_libre) sous [licence AGPL](LICENSE.txt).

Vous souhaitez y apporter des changements ou des améliorations ? Lisez notre [guide de contribution](CONTRIBUTING.md).