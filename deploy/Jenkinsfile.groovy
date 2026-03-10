pipeline {
    agent {
        docker {
            image 'docker:26.1.3'
            args '-v /var/run/docker.sock:/var/run/docker.sock'
        }
    }

    environment {
        IMAGE_TAG  = "latest"

        DOCKER_BIN = "docker"
        DOCKER_COMPOSE_BIN = "docker-compose"
        DOCKER_BASE_PATH = "./deploy/docker"
        DOCKER_COMPOSE_BASE_PATH = "./deploy"

        DATA_NETWORK    = credentials('DATA_NETWORK')
        TRAEFIK_NETWORK = credentials('TRAEFIK_NETWORK')
    }

    stages {

        stage('Clean Images') {
            steps {
                sh '''
                    ${DOCKER_COMPOSE_BIN} -f ${DOCKER_COMPOSE_BASE_PATH}/docker-compose.prod.yml down --rmi all -v --remove-orphans || true
                    ${DOCKER_BIN} rmi -f self-control-proxy:${IMAGE_TAG} || true
                '''
            }
        }

        stage('Build Images') {
            parallel {

                stage('Build Proxy Image') {
                    steps {
                        sh '''
                            ${DOCKER_BIN} rmi -f self-control-proxy:${IMAGE_TAG} || true
                            ${DOCKER_BIN} build \
                                -f ${DOCKER_BASE_PATH}/proxy.Dockerfile \
                                -t self-control-proxy:${IMAGE_TAG} \
                                ./proxy
                        '''
                    }
                }
            }
        }

        stage('Deploy') {
            steps {
                sh '''
                    # Generate .env file
                    cat > .env <<EOF
                    DATA_NETWORK=${DATA_NETWORK}
                    TRAEFIK_NETWORK=${TRAEFIK_NETWORK}
                    EOF
                '''

                sh '''
                    # Take down existing services
                    ${DOCKER_COMPOSE_BIN} -f ${DOCKER_COMPOSE_BASE_PATH}/docker-compose.prod.yml down || true
                '''

                sh '''
                    # Deploy new version
                    ${DOCKER_COMPOSE_BIN} -f ${DOCKER_COMPOSE_BASE_PATH}/docker-compose.prod.yml up -d --build
                '''
            }
            post {
                always {
                    sh 'rm -f .env'
                }
            }
        }

    }

    post {
        success { echo "Deployment successful" }
        failure { echo "Deployment failed" }
    }
}