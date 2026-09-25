plugins {
    alias(libs.plugins.android.application)
    alias(libs.plugins.kotlin.compose)
}

android {
    namespace = "io.github.gunsandsalvi.phoenix"
    compileSdk = 37

    defaultConfig {
        applicationId = "io.github.gunsandsalvi.phoenix"
        minSdk = 35
        targetSdk = 37
        versionCode = 1
        versionName = "0.7"
        ndk {
            abiFilters += "arm64-v8a"
        }
    }

    flavorDimensions += "mode"
    productFlavors {
        create("play") {
            dimension = "mode"
        }
        // The bench measures the phone with the engine's own kernels; it installs beside the game.
        create("bench") {
            dimension = "mode"
            applicationIdSuffix = ".bench"
            versionNameSuffix = "-bench"
        }
    }

    buildTypes {
        release {
            isMinifyEnabled = false
            // Signed with the debug key so a CI build installs directly; nothing is published.
            signingConfig = signingConfigs.getByName("debug")
        }
    }

    buildFeatures {
        compose = true
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }

    // The bench runs the world from the data the game reads, without the raw sources it was derived from.
    sourceSets.getByName("bench").assets.srcDir(layout.buildDirectory.dir("phx-assets"))
}

val phxData by tasks.registering(Sync::class) {
    from(rootProject.file("../data")) {
        exclude("sources/**")
    }
    into(layout.buildDirectory.dir("phx-assets/data"))
}

tasks.matching { it.name.startsWith("merge") && it.name.contains("Bench") && it.name.endsWith("Assets") }
    .configureEach { dependsOn(phxData) }

dependencies {
    implementation(platform(libs.compose.bom))
    implementation(libs.compose.material3)
    implementation(libs.compose.ui)
    implementation(libs.activity.compose)
    // UniFFI's Kotlin bindings reach the engine through JNA, whose Android build ships as an AAR.
    implementation("net.java.dev.jna:jna:${libs.versions.jna.get()}@aar")
}
