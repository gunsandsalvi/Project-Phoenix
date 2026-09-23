// AGP's built-in Kotlin compiles with the Kotlin Gradle plugin on the build's classpath; naming it pins the Kotlin
// version to the one the Compose compiler plugin below matches.
buildscript {
    dependencies {
        classpath(libs.kotlin.gradle.plugin)
    }
}

plugins {
    alias(libs.plugins.android.application) apply false
    alias(libs.plugins.kotlin.compose) apply false
}
