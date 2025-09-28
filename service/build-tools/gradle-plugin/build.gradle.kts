plugins {
    `java-gradle-plugin`
    `maven-publish`
    id("com.gradle.plugin-publish") version "1.2.1"
    kotlin("jvm") version "1.9.21"
}

group = "com.rootreal.linkml"
version = "2.0.0"

repositories {
    mavenCentral()
}

dependencies {
    implementation(gradleApi())
    implementation("com.fasterxml.jackson.core:jackson-databind:2.15.2")
    implementation("com.fasterxml.jackson.dataformat:jackson-dataformat-yaml:2.15.2")
    implementation("org.apache.commons:commons-exec:1.3")
    implementation("commons-io:commons-io:2.11.0")

    testImplementation("org.junit.jupiter:junit-jupiter:5.10.1")
    testImplementation("org.mockito:mockito-core:5.7.0")
    testImplementation(gradleTestKit())
}

gradlePlugin {
    website.set("https://github.com/simonckemper/rootreal")
    vcsUrl.set("https://github.com/simonckemper/rootreal.git")

    plugins {
        create("linkmlPlugin") {
            id = "com.rootreal.linkml"
            displayName = "LinkML Gradle Plugin"
            description = "Gradle plugin for LinkML schema validation and code generation"
            implementationClass = "com.rootreal.linkml.gradle.LinkMLPlugin"
            tags.set(listOf("linkml", "schema", "validation", "codegen", "data-modeling"))
        }
    }
}

java {
    sourceCompatibility = JavaVersion.VERSION_11
    targetCompatibility = JavaVersion.VERSION_11
    withSourcesJar()
    withJavadocJar()
}

tasks.test {
    useJUnitPlatform()
}

publishing {
    publications {
        create<MavenPublication>("maven") {
            groupId = project.group.toString()
            artifactId = project.name
            version = project.version.toString()

            from(components["java"])

            pom {
                name.set("LinkML Gradle Plugin")
                description.set("Gradle plugin for LinkML schema validation and code generation")
                url.set("https://github.com/simonckemper/rootreal")

                licenses {
                    license {
                        name.set("MIT License")
                        url.set("https://opensource.org/licenses/MIT")
                    }
                }

                developers {
                    developer {
                        id.set("rootreal")
                        name.set("RootReal Team")
                        email.set("support@textpast.com")
                    }
                }

                scm {
                    url.set("https://github.com/simonckemper/rootreal")
                    connection.set("scm:git:git://github.com/simonckemper/rootreal.git")
                    developerConnection.set("scm:git:ssh://github.com:simonckemper/rootreal.git")
                }
            }
        }
    }
}
