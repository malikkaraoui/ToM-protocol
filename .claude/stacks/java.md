---
stack: java
applies_to: ["*.java", "pom.xml", "build.gradle", "build.gradle.kts"]
loads_from: src/fr/CLAUDE.md §0 (Contexte projet)
figure: Marcel
---

# Stack — Java

> Depuis 1995. J'ai tout vu. J'ai tout survécu.
> Spring, Jakarta, Maven... J'ai mangé mes propres librairies et j'ai continué.
> *— Write once, run anywhere. Même si t'en as plus envie.* — Marcel ☕

## Principes

- **`Optional<T>` jamais `null`** pour les retours. Un `null` franchi sans
  vérification explicite est un bug qui attend son heure
- **`Consumer` / `Function` / `Supplier` / `Predicate`** pour les APIs
  fonctionnelles ; pas de re-création d'interfaces équivalentes
- **Exceptions toujours loggées** à la frontière métier ; jamais de
  `catch (Exception e) {}` silencieux
- **Immutabilité** : `final` par défaut sur les champs et variables locales
- **Records** pour les DTOs et value objects (Java 16+)

## Tooling par défaut

- Maven ou Gradle (hérite de l'existant, pas de migration gratuite)
- `checkstyle` + `spotbugs` pour le lint
- `jacoco` pour la couverture de tests
- JUnit 5 + AssertJ pour les tests (pas JUnit 4 sur du code neuf)

## Discipline de modules

- Packages par feature, pas par couche technique
- Visibilité minimale : `package-private` par défaut, `public` si justifié
- Pas de circular deps entre packages

## Gestion d'erreurs

- Exceptions métier = sous-classes explicites de `RuntimeException`
- `try-with-resources` pour toute ressource `AutoCloseable`
- Logger SLF4J + Logback, paramètres structurés (`log.info("user={}", id)`)
