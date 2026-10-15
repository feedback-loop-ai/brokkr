FROM ubuntu:24.04

ENV DEBIAN_FRONTEND=noninteractive
RUN apt-get update && apt-get install -y --no-install-recommends \
    acl build-essential ca-certificates curl git jq libcap-dev libicu74 \
    libkrb5-3 libssl3 libunwind8 meson ninja-build pkg-config python3 \
    rsync strace sudo time unzip xz-utils zip zlib1g \
    && rm -rf /var/lib/apt/lists/*

# The same verified bubblewrap release as setup-bubblewrap/action.yml.
RUN curl -fsSL --retry 3 -o /tmp/bwrap.tar.xz \
      https://github.com/containers/bubblewrap/releases/download/v0.11.0/bubblewrap-0.11.0.tar.xz \
    && echo '988fd6b232dafa04b8b8198723efeaccdb3c6aa9c1c7936219d5791a8b7a8646  /tmp/bwrap.tar.xz' | sha256sum -c - \
    && tar xJf /tmp/bwrap.tar.xz -C /tmp \
    && cd /tmp/bubblewrap-0.11.0 \
    && meson setup _build -Dselinux=disabled -Dman=disabled -Dbash_completion=disabled -Dzsh_completion=disabled \
    && ninja -C _build && install -m 0755 _build/bwrap /usr/local/bin/bwrap \
    && rm -rf /tmp/bwrap.tar.xz /tmp/bubblewrap-0.11.0

ARG RUNNER_VERSION=2.338.0
ARG RUNNER_SHA256=af4b794c1bc41d73d40535e3fe092a39f9679cd8d965954c2aca25a05ca41d32
RUN mkdir /runner && curl -fsSL --retry 3 -o /tmp/runner.tar.gz \
      "https://github.com/actions/runner/releases/download/v${RUNNER_VERSION}/actions-runner-linux-x64-${RUNNER_VERSION}.tar.gz" \
    && echo "${RUNNER_SHA256}  /tmp/runner.tar.gz" | sha256sum -c - \
    && tar xzf /tmp/runner.tar.gz -C /runner && rm /tmp/runner.tar.gz

RUN useradd --create-home --uid 1001 --shell /bin/bash runner \
    && echo 'runner ALL=(ALL) NOPASSWD:ALL' > /etc/sudoers.d/runner \
    && chmod 0440 /etc/sudoers.d/runner && chown -R runner:runner /runner
ENV PATH=/home/runner/.cargo/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin
USER runner
WORKDIR /home/runner
# Keep Rust outside /usr: the broker walks the protected system tree.
RUN curl -fsSL --retry 3 https://sh.rustup.rs -o /tmp/rustup-init.sh \
    && sh /tmp/rustup-init.sh -y --profile minimal --default-toolchain 1.98.0 \
    && rm /tmp/rustup-init.sh \
    && rustup component add --toolchain 1.98.0 clippy rustfmt \
    && rustup toolchain install 1.88.0 --profile minimal \
    && rustup toolchain install nightly-2026-09-05 --profile minimal --component llvm-tools-preview,clippy,rustfmt
RUN curl -fsSL --retry 3 -o /tmp/cov.tar.gz \
      https://github.com/taiki-e/cargo-llvm-cov/releases/download/v0.9.1/cargo-llvm-cov-x86_64-unknown-linux-gnu.tar.gz \
    && echo 'b3f68e625481fed9b16444174f3fa5ebcdbde4a1878803a35eabe2dcefcdc41a  /tmp/cov.tar.gz' | sha256sum -c - \
    && tar xzf /tmp/cov.tar.gz -C /home/runner/.cargo/bin && rm /tmp/cov.tar.gz

USER root
# The suite exercises Git's reftable support (Ubuntu 24.04 ships older Git).
RUN export RUSTUP_HOME=/home/runner/.rustup CARGO_HOME=/home/runner/.cargo; \
    apt-get update && apt-get install -y --no-install-recommends libcurl4-openssl-dev libexpat1-dev libssl-dev zlib1g-dev gettext \
    && rm -rf /var/lib/apt/lists/* \
    && curl -fsSL --retry 3 -o /tmp/git.tar.xz https://www.kernel.org/pub/software/scm/git/git-2.55.0.tar.xz \
    && echo '457fdb04dc8728e007d4688695e6912e6f680727920f2a40bf11eacc17505357  /tmp/git.tar.xz' | sha256sum -c - \
    && tar xJf /tmp/git.tar.xz -C /tmp \
    && make -C /tmp/git-2.55.0 -j8 prefix=/usr NO_TCLTK=YesPlease all \
    && make -C /tmp/git-2.55.0 prefix=/usr NO_TCLTK=YesPlease install \
    && rm -rf /tmp/git.tar.xz /tmp/git-2.55.0 \
    && chown -R runner:runner /home/runner/.cargo
COPY --chmod=755 ops/ci-runner/entrypoint.sh ops/ci-runner/job-started.sh /opt/brokkr-ci/
ENV ACTIONS_RUNNER_HOOK_JOB_STARTED=/opt/brokkr-ci/job-started.sh
WORKDIR /runner
ENTRYPOINT ["/opt/brokkr-ci/entrypoint.sh"]
