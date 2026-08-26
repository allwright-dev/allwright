plugins {
    `java-library`
    id("com.google.protobuf") version "0.9.4"
}

repositories {
    mavenCentral()
}

java {
    toolchain {
        languageVersion.set(JavaLanguageVersion.of(17))
    }
    withSourcesJar()
}

dependencies {
    api("io.grpc:grpc-netty-shaded:1.74.0")
    api("io.grpc:grpc-protobuf:1.74.0")
    api("io.grpc:grpc-stub:1.74.0")
    api("org.yaml:snakeyaml:2.2")
    compileOnly("javax.annotation:javax.annotation-api:1.3.2")
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
        id("grpc") {
            artifact = "io.grpc:protoc-gen-grpc-java:1.74.0"
        }
    }
    generateProtoTasks {
        all().configureEach {
            plugins {
                id("grpc")
            }
        }
    }
}

tasks.withType<JavaCompile>().configureEach {
    options.release.set(17)
}
