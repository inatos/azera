FROM jenkins/jenkins:lts

# Switch to root to install Docker CLI
USER root

# Install Docker CLI and dependencies
RUN apt-get update && apt-get install -y \
    apt-transport-https \
    ca-certificates \
    curl \
    gnupg \
    lsb-release \
    && curl -fsSL https://download.docker.com/linux/debian/gpg | gpg --dearmor -o /usr/share/keyrings/docker-archive-keyring.gpg \
    && echo "deb [arch=$(dpkg --print-architecture) signed-by=/usr/share/keyrings/docker-archive-keyring.gpg] https://download.docker.com/linux/debian $(lsb_release -cs) stable" > /etc/apt/sources.list.d/docker.list \
    && apt-get update \
    && apt-get install -y docker-ce-cli \
    && rm -rf /var/lib/apt/lists/*

# Add jenkins user to docker group (will be created on host mount)
RUN groupadd -f docker && usermod -aG docker jenkins

# Switch back to jenkins user for plugin installation
USER jenkins

# Install Jenkins plugins (use latest compatible versions)
RUN jenkins-plugin-cli --plugins \
    docker-workflow \
    docker-plugin \
    workflow-aggregator \
    git \
    pipeline-stage-view \
    blueocean \
    credentials-binding \
    timestamper \
    dark-theme \
    configuration-as-code

# Skip initial setup wizard (plugins already installed)
ENV JAVA_OPTS="-Djenkins.install.runSetupWizard=false"

# Copy JCasC config for dark theme (to ref folder, not jenkins_home which gets mounted)
COPY --chown=jenkins:jenkins casc.yaml /usr/share/jenkins/ref/casc.yaml
ENV CASC_JENKINS_CONFIG=/usr/share/jenkins/ref/casc.yaml

# Copy init scripts
COPY --chown=jenkins:jenkins init.groovy.d/ /usr/share/jenkins/ref/init.groovy.d/
