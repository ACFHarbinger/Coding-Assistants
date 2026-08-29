import java.io.FileInputStream
import java.util.Properties

plugins {
    id("com.android.application")
    id("org.jetbrains.kotlin.android")
    id("org.jetbrains.kotlin.plugin.compose")
    id("org.jetbrains.kotlin.plugin.serialization")
    id("org.jlleitschuh.gradle.ktlint")
}

base {
    archivesName.set("coding-assistants-companion")
}

android {
    namespace = "com.codingassistants.remotelauncher"
    compileSdk = 34

    defaultConfig {
        applicationId = "com.codingassistants.remotelauncher"
        minSdk = 24
        targetSdk = 34
        // Monotonically derived: major*10000 + minor*100 + patch; set by just release::bump
        versionCode = 100
        // Semver string; set by just release::bump
        versionName = "0.1.0"

        testInstrumentationRunner = "androidx.test.runner.AndroidJUnitRunner"
    }

    val keystorePropertiesFile = rootProject.file("keystore.properties")
    val keystoreProperties = Properties()
    if (keystorePropertiesFile.exists()) {
        keystoreProperties.load(FileInputStream(keystorePropertiesFile))
    }

    val releaseStoreFilePath =
        keystoreProperties.getProperty("storeFile")
            ?: System.getenv("ANDROID_KEYSTORE_FILE")
            ?: "keystore/release.jks"

    val releaseStorePassword =
        keystoreProperties.getProperty("storePassword")
            ?: System.getenv("ANDROID_KEYSTORE_PASSWORD")

    val releaseKeyAlias =
        keystoreProperties.getProperty("keyAlias")
            ?: System.getenv("ANDROID_KEY_ALIAS")

    val releaseKeyPassword =
        keystoreProperties.getProperty("keyPassword")
            ?: System.getenv("ANDROID_KEY_PASSWORD")

    signingConfigs {
        create("release") {
            val resolvedStoreFile =
                releaseStoreFilePath.let { path ->
                    val f = file(path)
                    if (f.isAbsolute && f.exists()) {
                        f
                    } else if (rootProject.file(path).exists()) {
                        rootProject.file(path)
                    } else {
                        rootProject.file(path)
                    }
                }

            val hasAllCredentials =
                resolvedStoreFile.exists() &&
                    !releaseStorePassword.isNullOrBlank() &&
                    !releaseKeyAlias.isNullOrBlank() &&
                    !releaseKeyPassword.isNullOrBlank()

            if (hasAllCredentials) {
                storeFile = resolvedStoreFile
                storePassword = releaseStorePassword
                keyAlias = releaseKeyAlias
                keyPassword = releaseKeyPassword
            } else {
                // If release signing is not fully configured, fail release packaging/signing tasks with a clear diagnostic
                gradle.taskGraph.whenReady {
                    val isReleasePackagingTask =
                        allTasks.any { task ->
                            task.name in
                                setOf(
                                    "assembleRelease",
                                    "bundleRelease",
                                    "validateSigningRelease",
                                    "packageRelease",
                                    "packageReleaseApk",
                                    "packageReleaseBundle",
                                    "signReleaseBundle",
                                )
                        }
                    if (isReleasePackagingTask) {
                        val missing = mutableListOf<String>()
                        if (!resolvedStoreFile.exists()) {
                            val path = resolvedStoreFile.absolutePath
                            missing.add("storeFile (file not found at '$path')")
                        }
                        if (releaseStorePassword.isNullOrBlank()) {
                            missing.add("storePassword / ANDROID_KEYSTORE_PASSWORD")
                        }
                        if (releaseKeyAlias.isNullOrBlank()) {
                            missing.add("keyAlias / ANDROID_KEY_ALIAS")
                        }
                        if (releaseKeyPassword.isNullOrBlank()) {
                            missing.add("keyPassword / ANDROID_KEY_PASSWORD")
                        }

                        val missingSummary = missing.joinToString(", ")
                        throw GradleException(
                            "Release signing configuration is missing or incomplete:\n" +
                                "  Missing: $missingSummary\n" +
                                "Please provide android/keystore.properties " +
                                "(with storeFile, storePassword, keyAlias, keyPassword)\n" +
                                "or set environment variables: " +
                                "ANDROID_KEYSTORE_PASSWORD, ANDROID_KEY_ALIAS, ANDROID_KEY_PASSWORD.",
                        )
                    }
                }
            }
        }
    }

    buildTypes {
        release {
            isMinifyEnabled = false
            signingConfig = signingConfigs.getByName("release")
            proguardFiles(getDefaultProguardFile("proguard-android-optimize.txt"), "proguard-rules.pro")
        }
    }
    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }
    kotlinOptions {
        jvmTarget = "17"
    }
    buildFeatures {
        compose = true
    }
}

dependencies {
    implementation("androidx.core:core-ktx:1.12.0")
    implementation("androidx.lifecycle:lifecycle-runtime-ktx:2.7.0")
    implementation("androidx.activity:activity-compose:1.8.2")

    // Jetpack Compose
    implementation(platform("androidx.compose:compose-bom:2024.01.00"))
    implementation("androidx.compose.ui:ui")
    implementation("androidx.compose.ui:ui-graphics")
    implementation("androidx.compose.ui:ui-tooling-preview")
    implementation("androidx.compose.material3:material3")
    implementation("androidx.compose.material:material-icons-extended")

    // Navigation
    implementation("androidx.navigation:navigation-compose:2.7.6")

    // ViewModel
    implementation("androidx.lifecycle:lifecycle-viewmodel-compose:2.7.0")

    // Coroutines
    implementation("org.jetbrains.kotlinx:kotlinx-coroutines-android:1.7.3")

    // JSON
    implementation("org.jetbrains.kotlinx:kotlinx-serialization-json:1.6.2")

    // Networking (for TCP communication)
    implementation("io.ktor:ktor-network:2.3.7")

    debugImplementation("androidx.compose.ui:ui-tooling")
    debugImplementation("androidx.compose.ui:ui-test-manifest")
}
