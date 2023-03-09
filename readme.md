# DS Proxy

## Contexte

DS Proxy est un proxy HTTP de chiffrement en streaming. Il est utilisé sur [démarches-simplifées](https://github.com/demarches-simplifiees/demarches-simplifiees.fr) avec un backend Openstack Swift mais devrait être compatible avec le protocol S3.
Il permet de se prémunir d'accès non authorisé aux espaces de stockages mutualisés de type object storage en y stockant uniquement de la donnée chiffrée.

Fonctionnalités :
- chiffre et déchiffre de manière transparente pour le client des fichiers en http en les les stockants sur l'object storage
- chiffre et stocke en local de gros fichier (`/local/`) depuis un envoi http
- chiffrer et dechiffrer des fichiers sur le système de fichier
- est performant
- supporte de multiple clés de chiffrement pour se conformer à une politique de péremption de clés
- possède une url de health check `/ping` qui renvoie une 404 si le fichier `maintenance` est présent à coté du binaire.

## Pour commencer

prérequis: 
- [rust](rust-lang.org) dans la version préciser par le fichier `Cargo.toml`
- et dans la cadre de la démo [node](https://nodejs.org)

puis lancer le script `launch_demo.sh` qui compilera le proxy, le démarrera, et qui lancera un faux backend object storage en node.

## Installation

- compiler le proxy pour la production: `cargo build --release`, le binaire se trouve à présent ici : `target/release/ds_proxy`
- placer le binaire sur votre server et utiliser votre système habituel pour le superviser
exemple d'un fichier service minimal de supervision par systemd.

```
[Unit]
Description=DS Proxy Service
After=network.target

[Service]
ExectStart=/usr/bin/ds_proxy proxy --password-file /var/ds_proxy/password --salt a_32_charactere_long_salt_123456 --keyring-file /var/ds_proxy/keyring.toml  --local-encryption-directory /var/ds_proxy/local_encryption/ --address 0.0.0.0:4444 --upstream_url 'https://my-storage-object.com'

Environment=RUST_LOG="actix_web=info"
...
```

### Garder le password en mémoire

Pour éviter que le password ne reste sur le disque et en suivant https://www.netmeister.org/blog/passing-passwords.html, nous utilisons `mkfifo` pour créer un named pipe qui nous permet de transmettre le mot passe en restant en mémoire.
En voici le principe :
```
mkfifo -m 0600 password_file
systemctl start ds_proxy
systemd-ask-password > password_file
rm -f password_file
```

## Dans le détail

### Algo
DS Proxy utilise actuellement l'algorithme de chiffrement [xchacha20poly1305](https://doc.libsodium.org/secret-key_cryptography/aead/chacha20-poly1305/xchacha20-poly1305_construction) proposé par la librairie [sodium](https://doc.libsodium.org/) dont l'interface est portée en rust par [sodiumoxide](https://github.com/sodiumoxide/sodiumoxide).

Les clés de chiffrement sont stockés sur un fichier `keyring.toml`. Ce fichier est lui même chiffré à l'aide d'un mot de passe maître et d'un sel.

## Comment contribuer ?

**ds_proxy** est un [logiciel libre](https://fr.wikipedia.org/wiki/Logiciel_libre) sous [licence AGPL](LICENSE.txt).

Vous souhaitez y apporter des changements ou des améliorations ? Lisez notre [guide de contribution](CONTRIBUTING.md).
