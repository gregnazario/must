plugins {
    kotlin("jvm") version "1.9.24"
    `maven-publish`
    `signing`
}

group = "com.myorg"
version = "0.1.0"

kotlin {
    jvmToolchain(17)
    explicitApi()
}

java {
    withJavadocJar()
    withSourcesJar()
}

publishing {
    publications {
        create<MavenPublication>("mavenKotlin") {
            from(components["kotlin"])
            pom {
                name.set("my-kotlin-lib")
                description.set("A sample Kotlin library")
                url.set("https://github.com/myorg/my-kotlin-lib")
                licenses {
                    license {
                        name.set("MIT License")
                        url.set("https://opensource.org/licenses/MIT")
                    }
                }
            }
        }
    }
}

tasks.test {
    useJUnitPlatform()
}

repositories {
    mavenCentral()
}

dependencies {
    testImplementation(kotlin("test"))
}
