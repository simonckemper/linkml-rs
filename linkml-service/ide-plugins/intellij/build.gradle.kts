plugins {
    id("java")
    id("org.jetbrains.kotlin.jvm") version "1.9.21"
    id("org.jetbrains.intellij") version "1.16.1"
}

group = "com.rootreal"
version = "2.0.0"

repositories {
    mavenCentral()
}

// Configure Gradle IntelliJ Plugin
intellij {
    version.set("2023.3.2")
    type.set("IC") // IntelliJ IDEA Community Edition

    plugins.set(listOf(
        "org.jetbrains.plugins.yaml",
        "com.intellij.java"
    ))
}

dependencies {
    implementation("org.yaml:snakeyaml:2.2")
    implementation("com.google.code.gson:gson:2.10.1")
    implementation("com.networknt:json-schema-validator:1.0.87")
    testImplementation("org.junit.jupiter:junit-jupiter:5.10.1")
}

tasks {
    // Set the JVM compatibility versions
    withType<JavaCompile> {
        sourceCompatibility = "17"
        targetCompatibility = "17"
    }

    withType<org.jetbrains.kotlin.gradle.tasks.KotlinCompile> {
        kotlinOptions.jvmTarget = "17"
    }

    patchPluginXml {
        sinceBuild.set("233")
        untilBuild.set("241.*")

        changeNotes.set("""
            <h3>2.0.0</h3>
            <ul>
                <li>Complete LinkML support with 100% feature parity</li>
                <li>Syntax highlighting for LinkML schemas</li>
                <li>Real-time validation</li>
                <li>Code completion</li>
                <li>Code generation to multiple targets</li>
                <li>Schema visualization</li>
                <li>Refactoring support</li>
                <li>Quick fixes and inspections</li>
                <li>Integration with RootReal LinkML service</li>
            </ul>
        """.trimIndent())
    }

    signPlugin {
        certificateChain.set(System.getenv("CERTIFICATE_CHAIN"))
        privateKey.set(System.getenv("PRIVATE_KEY"))
        password.set(System.getenv("PRIVATE_KEY_PASSWORD"))
    }

    publishPlugin {
        token.set(System.getenv("PUBLISH_TOKEN"))
    }
}

tasks.test {
    useJUnitPlatform()
}
