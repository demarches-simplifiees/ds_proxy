# Comment contribuer

ds_proxy est un [logiciel libre](https://fr.wikipedia.org/wiki/Logiciel_libre). Vous pouvez lire et modifier son code-source, sous les termes de la licence AGPL.

Si vous souhaitez apporter des améliorations à ds_proxy, c’est possible !

Le mieux pour cela est de **proposer une modification dans la base de code principale**. Une fois acceptée, votre améliorations sera ainsi disponible pour l’ensemble des utilisateurs de ds_proxy.

Voici la marche à suivre recommandée pour effectuer une modification.

## 1. Discuter de l’amélioration

La première étape est généralement de discuter de l’amélioration que vous proposez (s’il ne s’agit pas d’un changement trivial, comme la correction d’une coquille).

Pour cela, [créez une nouvelle issue](https://github.com/betagouv/ds_proxy/issues/new) concernant votre proposition. Autant que possible, indiquez clairement votre besoin, votre cas d’usage – et éventuellement comment vous pensez déjà le résoudre.

Nous pouvons alors discuter, pour vérifier que le besoin exprimé correspond à l’usage de ds_proxy, proposer éventuellement des alternatives, et se mettre d’accord sur une implémentation technique pertinente.

## 2. Proposer du code

Une fois que la discussion est établie, et que les éléments techniques sont dégrossis, vous pouvez proposer des changements au code. Pour cela, effectuez vos modifications en local, et [ouvrez une Pull Request](https://github.com/betagouv/ds_proxy/issues/new) avec les changements que vous souhaitez apporter.

Quelques conseils : pensez à bien décrire l’objectif et l’implémentation de votre PR au moment de la créer. Et si vos changements sont importants, découpez-les en plusieurs petites PRs successives, qui seront plus faciles à relire. N’oubliez pas d’ajouter des tests automatisés pour vous assurer que vos changements fonctionnent bien.

Chaque PR ouverte déclenche l’exécution des tests automatisés, et la vérification du format du code. Si vos tests ou votre formattage est en rouge, corrigez les erreurs avant de continuer.

Une personne de l’équipe de développement fera une relecture, en demandant éventuellement des détails ou des changements. Si personne n’a réagi au bout de 5 jours, n’hésitez pas à relancer en ajoutant un commentaire à la PR.

## 3. Intégration

Une fois votre PR approuvée, elle sera intégrée dans la base de code principale.

