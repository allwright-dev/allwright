import org.gradle.external.javadoc.StandardJavadocDocletOptions

plugins {
    `java-library`
    `maven-publish`
    signing
    id("com.google.protobuf") version "0.9.4"
}

group = "dev.allwright"
version = providers.gradleProperty("allwrightVersion")
    .orElse(providers.environmentVariable("ALLWRIGHT_VERSION"))
    .orElse("0.0.58")
    .get()
description = "Playwright-style Java client for the allwright engine."

repositories {
    mavenCentral()
}

java {
    toolchain {
        languageVersion.set(JavaLanguageVersion.of(21))
    }
    withSourcesJar()
    withJavadocJar()
}

dependencies {
    api("io.grpc:grpc-netty-shaded:1.74.0")
    api("io.grpc:grpc-protobuf:1.74.0")
    api("io.grpc:grpc-stub:1.74.0")
    api("org.yaml:snakeyaml:2.2")
    compileOnly("javax.annotation:javax.annotation-api:1.3.2")
    testImplementation(platform("org.junit:junit-bom:5.11.4"))
    testImplementation("org.junit.jupiter:junit-jupiter-api")
    testRuntimeOnly("org.junit.jupiter:junit-jupiter-engine")
    testRuntimeOnly("org.junit.platform:junit-platform-launcher")
}

sourceSets {
    main {
        proto {
            srcDir("../proto")
        }
    }
}

protobuf {
    protoc {
        artifact = "com.google.protobuf:protoc:3.25.5"
    }
    plugins {
        create("grpc") {
            artifact = "io.grpc:protoc-gen-grpc-java:1.74.0"
        }
    }
    generateProtoTasks {
        all().configureEach {
            plugins {
                create("grpc")
            }
        }
    }
}

tasks.withType<JavaCompile>().configureEach {
    options.release.set(21)
}

tasks.withType<Javadoc>().configureEach {
    val options = options as StandardJavadocDocletOptions
    options.addBooleanOption("Xdoclint:none", true)
    options.encoding = "UTF-8"
}

tasks.test {
    useJUnitPlatform()
}

publishing {
    publications {
        create<MavenPublication>("mavenJava") {
            from(components["java"])
            artifactId = "allwright"

            pom {
                name.set("allwright Java")
                description.set(project.description)
                url.set("https://github.com/allwright-dev/allwright")

                licenses {
                    license {
                        name.set("MIT License")
                        url.set("https://github.com/allwright-dev/allwright/blob/main/LICENSE")
                    }
                }

                developers {
                    developer {
                        id.set("allwright-dev")
                        name.set("allwright contributors")
                        url.set("https://github.com/allwright-dev/allwright")
                    }
                }

                scm {
                    connection.set("scm:git:https://github.com/allwright-dev/allwright.git")
                    developerConnection.set("scm:git:ssh://git@github.com/allwright-dev/allwright.git")
                    url.set("https://github.com/allwright-dev/allwright")
                }
            }
        }
    }

    repositories {
        maven {
            name = "CentralPortal"
            val releasesUrl = "https://ossrh-staging-api.central.sonatype.com/service/local/staging/deploy/maven2/"
            val snapshotsUrl = "https://central.sonatype.com/repository/maven-snapshots/"
            url = uri(if (version.toString().endsWith("SNAPSHOT")) snapshotsUrl else releasesUrl)

            credentials {
                username = providers.gradleProperty("ossrhUsername")
                    .orElse(providers.environmentVariable("OSSRH_USERNAME"))
                    .orNull
                password = providers.gradleProperty("ossrhPassword")
                    .orElse(providers.environmentVariable("OSSRH_PASSWORD"))
                    .orNull
            }
        }
    }
}

signing {
    val signingKey = providers.gradleProperty("signingKey")
        .orElse(providers.environmentVariable("SIGNING_KEY"))
        .orNull
    val signingPassword = providers.gradleProperty("signingPassword")
        .orElse(providers.environmentVariable("SIGNING_PASSWORD"))
        .orNull

    if (!signingKey.isNullOrBlank()) {
        useInMemoryPgpKeys(signingKey, signingPassword)
        sign(publishing.publications["mavenJava"])
    }
}
