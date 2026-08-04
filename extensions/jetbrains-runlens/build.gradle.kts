plugins {
    id("org.jetbrains.intellij.platform") version "2.2.1"
    kotlin("jvm") version "1.9.25"
}

group = "runlens"
version = "0.1.0"

repositories {
    mavenCentral()
    intellijPlatform { defaultRepositories() }
}

dependencies {
    intellijPlatform {
        intellijIdeaCommunity("2023.1")
        pluginVerifier()
        zipSigner()
    }
}

intellijPlatform {
    pluginConfiguration {
        name = "RunLens"
        version = project.version.toString()
        description = "Local-first developer flight recorder integration for IntelliJ"
        ideaVersion {
            sinceBuild = "231"
            untilBuild = "242.*"
        }
        vendor("RunLens Contributors", "https://github.com/runlens/runlens")
    }
    signing {
        certificateChain = ""
        privateKey = ""
        password = ""
    }
    publishing {
        token = ""
    }
}

kotlin { jvmToolchain(17) }
