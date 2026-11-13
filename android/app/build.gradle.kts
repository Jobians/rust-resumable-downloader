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
        versionCode = 1
        versionName = "1.0"
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
