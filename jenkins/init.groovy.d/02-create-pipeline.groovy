import jenkins.model.*
import org.jenkinsci.plugins.workflow.job.WorkflowJob
import org.jenkinsci.plugins.workflow.cps.CpsFlowDefinition

def instance = Jenkins.getInstance()

def existingJob = instance.getItem("azera-pipeline")

// Read host project path and convert Windows paths to Docker-compatible Unix format
// e.g. C:\Users\foo\project -> /c/Users/foo/project
def hostPath = System.getenv('PROJECT_HOST_PATH') ?: '/project'
if (hostPath =~ /^[A-Za-z]:/) {
    hostPath = '/' + hostPath[0].toLowerCase() + hostPath.substring(2).replace('\\', '/')
}
println "📂 Project host path: ${hostPath}"

def pipelineScript = '''
pipeline {
    agent none

    environment {
        RUST_BACKTRACE = '1'
        CARGO_TERM_COLOR = 'always'
        PROJECT_PATH = '__PROJECT_PATH__'
    }

    stages {
        stage('Backend') {
            agent {
                docker {
                    image 'rust:latest'
                    args "-v ${PROJECT_PATH}:/workspace -v cargo-cache:/usr/local/cargo/registry"
                }
            }
            steps {
                sh 'cd /workspace/backend && cargo test --no-fail-fast && cargo build --release'
            }
        }

        stage('Frontend') {
            agent {
                docker {
                    image 'oven/bun:latest'
                    args "-v ${PROJECT_PATH}:/workspace"
                }
            }
            steps {
                sh 'cd /workspace/frontend && bun install && bun test && bun run check && bun run build'
            }
        }
    }

    post {
        success { echo '🎉 Pipeline completed successfully!' }
        failure { echo '❌ Pipeline failed.' }
    }
}
'''.replace('__PROJECT_PATH__', hostPath)

if (existingJob == null) {
    def job = instance.createProject(WorkflowJob.class, "azera-pipeline")
    job.setDefinition(new CpsFlowDefinition(pipelineScript, true))
    job.save()
    println "✅ Created azera-pipeline job"
} else {
    existingJob.setDefinition(new CpsFlowDefinition(pipelineScript, true))
    existingJob.save()
    println "🔄 Updated azera-pipeline job"
}
