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
}

dependencies {
    implementation(platform(libs.compose.bom))
    implementation(libs.compose.material3)
    implementation(libs.compose.ui)
    implementation(libs.activity.compose)
}
