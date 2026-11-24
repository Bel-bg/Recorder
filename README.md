# Recoder

Application Android minimaliste de pratique orale de l'anglais. Vous écoutez un extrait, vous ouvrez Recoder, vous enregistrez votre répétition, puis vous gardez la prise. Tout reste sur le téléphone.

Recoder est écrit en Rust avec [Dioxus](https://dioxuslabs.com) Mobile et ne fonctionne pas sans son microphone ni son stockage local. Il n'y a ni compte, ni serveur, ni connexion Internet obligatoire.

## Fonctionnalités

- Écran d'enregistrement affiché dès l'ouverture de l'application.
- Enregistrement réel du microphone Android, chronomètre et forme d'onde alimentée par l'amplitude du signal.
- Trois états du bouton principal: démarrage, enregistrement en cours, puis une prise terminée que l'on peut écouter, garder ou supprimer.
- Autorisation du microphone demandée au moment de l'enregistrement, avec une explication avant la boîte de dialogue Android.
- Historique groupé par jour, avec lecture et suppression définitive du fichier audio.
- Lecteur audio réel: lecture, pause, reprise, arrêt, barre de progression, temps écoulé et durée totale.
- Statistiques calculées uniquement à partir des prises enregistrées: nombre de prises, temps de parole, durée moyenne, jours actifs, activité du jour et des sept derniers jours.
- Objectif quotidien en minutes, modifiable et conservé localement.
- Navigation par barre inférieure et par gestes horizontaux.

## Technologies

| Élément | Choix |
| --- | --- |
| Langage | Rust (edition 2021) |
| Interface | Dioxus 0.7.10 Mobile, RSX et CSS |
| Capture et lecture audio | cpal 0.18, backend AAudio sur Android |
| Stockage audio | fichiers WAV 16 bits mono dans le stockage privé de l'application |
| Métadonnées | SQLite via rusqlite (bibliothèque embarquée) |
| Icônes | [Papicons](https://github.com/PapillonApp/Papicons), licence MIT, SVG inclus dans le binaire |
| Build Android | Dioxus CLI, Gradle et Android NDK |

Aucune donnée ne quitte l'appareil: pas de télémétrie, pas de backend, aucune permission de stockage externe.

## Prérequis

- Rust stable récent (testé avec 1.95) et les cibles Android:

  ```bash
  rustup target add aarch64-linux-android armv7-linux-androideabi i686-linux-android x86_64-linux-android
  ```

- JDK 17.
- Android SDK avec les outils en ligne de commande, la plateforme `android-35` et le NDK `28.2.13676358`.
- Dioxus CLI 0.7.10:

  ```bash
  curl -sSL https://github.com/DioxusLabs/dioxus/releases/download/v0.7.10/dx-x86_64-unknown-linux-gnu.tar.gz | tar xz -C "$HOME/.cargo/bin"
  ```

Variables d'environnement attendues:

```bash
export JAVA_HOME=/chemin/vers/jdk-17
export ANDROID_HOME=$HOME/Android/Sdk
export NDK_HOME=$ANDROID_HOME/ndk/28.2.13676358
```

## Installation

```bash
git clone https://github.com/Bel-bg/Recorder.git
cd Recorder
cargo test -p recoder-core
```

Les tests couvrent le stockage SQLite, les statistiques et le formatage des durées. Ils s'exécutent sur la machine de développement, sans Android.

## Lancement en développement

Branchez un appareil Android en mode développeur (débogage USB activé) ou démarrez un émulateur, puis:

```bash
dx serve --platform android
```

La CLI Dioxus compile la bibliothèque pour l'architecture de l'appareil, construit l'application Gradle et l'installe.

## Compilation de l'APK

En local:

```bash
dx bundle --platform android --release --package-types apk --target aarch64-linux-android
```

L'APK apparaît sous `target/dx/recoder/release/android/app/app/build/outputs/apk/debug/app-debug.apk`.

La cible `aarch64-linux-android` correspond aux téléphones Android 64 bits, soit la quasi totalité du parc actuel. Sans `--target`, la CLI Dioxus compile pour l'architecture de la machine hôte, ce qui produit un APK inutilisable sur un téléphone.

## Récupérer l'APK depuis GitHub Actions

1. Poussez une modification sur la branche `main`, ou lancez le workflow manuellement depuis l'onglet **Actions**.
2. Ouvrez le dernier run nommé **Android APK**.
3. Dans la section **Artifacts** en bas du résumé du run, téléchargez `recoder-apk`.
4. Décompressez l'archive pour obtenir `recoder.apk`, puis installez-la sur le téléphone (le téléphone demandera d'autoriser l'installation depuis cette source).

L'APK n'est pas signé pour une publication Play Store: il est destiné aux tests personnels.

## Structure du projet

```
Recorder/
├── Cargo.toml                 application Dioxus et workspace
├── Dioxus.toml                nom, identifiant Android, permission micro, minSdk
├── assets/
│   ├── main.css               jetons de design, mise en page, animations
│   └── papicons/              icônes Papicons (MIT) incluses à la compilation
├── core/                      crate recoder-core, sans dépendance interface
│   └── src/
│       ├── models.rs          modèle d'une prise
│       ├── storage.rs         SQLite, prises et préférences
│       ├── stats.rs           résumé et activité hebdomadaire
│       └── format.rs          durées et libellés
└── src/
    ├── main.rs                point d'entrée
    ├── app.rs                 coquille, gestes, transitions
    ├── state.rs               état partagé et actions métier
    ├── storage.rs             chemins du stockage privé
    ├── platform/              pont JNI (permission, dossier de fichiers)
    ├── audio/                 capture cpal, écriture WAV, lecteur
    ├── pages/                 enregistrement, historique, statistiques
    └── ui/                    composants: navigation, lecteur, icônes, forme d'onde
```

## Choix techniques

- La capture écrit les échantillons dans un fichier WAV par un fil dédié, sans jamais garder une prise entière en mémoire.
- La lecture ne décode que le fichier sélectionné et suit sa position via un compteur atomique partagé avec le callback audio.
- Le pont Android passe par `ndk-context`, fourni par le WebView de Dioxus, pour atteindre l'activité et demander `RECORD_AUDIO`.
- L'objectif quotidien et les métadonnées des prises vivent dans une base SQLite unique dans le dossier privé de l'application.

## Licence des icônes

Les icônes Papicons sont distribuées sous licence MIT par l'association Papillon. Le texte de licence est conservé dans `assets/papicons/LICENSE`.
