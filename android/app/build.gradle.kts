plugins {
    id("com.android.application")
}

android {
    namespace = "dev.jt.rget"
    compileSdk = 34

    defaultConfig {
        applicationId = "dev.jt.rget"
        minSdk = 21
        targetSdk = 34
        versionCode = 2
        versionName = "0.1.1"
    }

    buildTypes {
        release {
            isMinifyEnabled = false
        }
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }
}
