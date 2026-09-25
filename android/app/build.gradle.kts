import javax.inject.Inject

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
}

/**
 * The data the game reads, without the raw sources it was derived from, laid out as the bench's `data` asset, and the
 * full-load bench's declared volumes as its `load` asset.
 */
abstract class PhxData : DefaultTask() {
    @get:Internal
    abstract val source: DirectoryProperty

    @get:Internal
    abstract val load: DirectoryProperty

    @get:InputFiles
    @get:PathSensitive(PathSensitivity.RELATIVE)
    val inputs: FileTree
        get() = source.asFileTree.matching { exclude("sources/**") }.plus(load.asFileTree)

    @get:OutputDirectory
    abstract val output: DirectoryProperty

    @get:Inject
    abstract val files: FileSystemOperations

    @TaskAction
    fun copy() {
        files.sync {
            into(output)
            from(source) {
                exclude("sources/**")
                into("data")
            }
            from(load) { into("load") }
        }
    }
}

val phxData = tasks.register<PhxData>("phxData") {
    source.set(rootProject.layout.projectDirectory.dir("../data"))
    load.set(rootProject.layout.projectDirectory.dir("../perf/load"))
}

// The bench runs the world from the data; the variant API carries the task's dependency to the assets it feeds.
androidComponents {
    onVariants(selector().withFlavor("mode" to "bench")) { variant ->
        variant.sources.assets?.addGeneratedSourceDirectory(phxData, PhxData::output)
    }
}

dependencies {
    implementation(platform(libs.compose.bom))
    implementation(libs.compose.material3)
    implementation(libs.compose.ui)
    implementation(libs.activity.compose)
    // UniFFI's Kotlin bindings reach the engine through JNA, whose Android build ships as an AAR.
    implementation("net.java.dev.jna:jna:${libs.versions.jna.get()}@aar")
}
