# Juliette Pluto

_Adversarial robustness for frontier AI systems at Google DeepMind. Previously: trusted execution environments, privacy-preserving ML, and web engineering._

[j@jul.sh](mailto:j@jul.sh)

## Experience

### **Google DeepMind** · Staff Software Engineer
#### 2024-05 — present · NYC · promoted to Senior 2024, Staff 2026

* **Security evaluations for model selection:** Started DeepMind’s first effort to defend against prompt injection, built Gemini’s first adaptive eval in the post-training pipeline and early post-training defenses ([arXiv:2505.14534](https://arxiv.org/abs/2505.14534)). Now lead prompt injection evaluations and manage two engineers.
* **Prompt injection robustness for Gemini:** Led an analysis of human red-teaming and real-world attacks showing that our automated attackers were collapsing onto a narrow set of strategies, and drove the shift to strategy-diverse adversarial training data. Co-wrote the prompt injection section of the [Gemini 4 Argon Safety & Security Report](https://storage.googleapis.com/deepmind-media/gemini/gemini_4_safety_report.pdf), where Argon ranked as the most robust frontier model on the Gray Swan’s benchmark.
* **User Instruction Classifier:** Built a novel [monitor](https://arxiv.org/abs/2505.14534) that checks each action an agent proposes against the user’s request without exposing the monitor to untrusted data. Advised on the production defense built on the same principle for Gemini computer use; Anthropic’s Claude Code auto mode, released ten months after our paper, uses the same approach ([Anthropic, 2026](https://www.anthropic.com/engineering/claude-code-auto-mode)).
* **Training mitigations for an enterprise deployment:** Picked by leadership to close a critical prompt injection robustness gap for a multi-billion-dollar enterprise contract. Analyzed failure modes and created targeted training mitigations, doubling robustness on an internal benchmark. Advised customer's Head of ML Safety helping secure final acceptance.
* **Defending Google Search:** Identified unaddressed attack surfaces in Google Search before the wider organization was aware of them. Directed a cross-org technical response and co-authored sections of the Google-wide Prompt Injection Playbook.

### **Google** · Software Engineer
#### 2020-04 — 2024-05 · London, then New York

* **Trusted execution environments ([Project Oak](https://github.com/project-oak/oak)):** Core contributor to Google’s open-source confidential computing stack. Designed and built its orchestrator, which runs the attested workloads in [Gboard’s confidential federated analytics launch](https://arxiv.org/abs/2404.10764).
* **Browser-to-enclave encryption:** Engineered attested browser-to-enclave end-to-end encryption and prevented critical zero-day vulnerabilities during infrastructure design.

### **N26** · Software Engineer
#### 2018-01 — 2020-03 · Berlin

Joined the web platform rebuild as one of four engineers and helped grow the team past 20 as N26 became a unicorn. Led product work across the stack, mentored engineers, and cut production errors by 30%, time-to-first-meaningful-paint by 20%, and time-to-interactive by 1.4s.

### **Language Academy** · Lead Engineer
#### 2016-08 — 2017-12 · Berlin

### **Clue** · Web Developer & Design Researcher
#### 2015-10 — 2016-08 · Berlin

## Papers

* *The Attacker Moves Second: Stronger Adaptive Attacks Bypass Defenses Against LLM Jailbreaks and Prompt Injections.* Nasr, Carlini, Sitawarin, Schulhoff, Hayes, Ilie, **Pluto**, Song, Chaudhari, Shumailov, Thakurta, Xiao, Terzis, Tramèr. [USENIX Security 2026](https://www.usenix.org/conference/usenixsecurity26/presentation/nasr), 2026.
* *Lessons from Defending Gemini Against Indirect Prompt Injections.* Shi, Lin, Song, Hayes, Shumailov, Yona, **Pluto**, Pappu, Choquette-Choo, Nasr, Sitawarin, Gibson, Terzis, Flynn. [arXiv:2505.14534](https://arxiv.org/abs/2505.14534), 2025.
* *Gemini 2.5: Pushing the Frontier with Advanced Reasoning, Multimodality, Long Context, and Next Generation Agentic Capabilities.* Gemini Team, incl. **Pluto**. [arXiv:2507.06261](https://arxiv.org/abs/2507.06261), 2025.

## Talks

* *DICE Attestation on AMD SEV-SNP.* With Ivan Petrov. [OC3 2024](https://www.youtube.com/watch?v=LaT_vgz3Dd4).
* *Beyond the Paper: Operationalizing Auto Red Teaming for Measurable Model Evaluation & Development.* AI Red Team Colloquium (invite-only, hosted by Microsoft), 2026.

## Education

### **Humboldt University**
#### 2013-10 — 2016-03 · Berlin

## Recognition

### **Google Developer Expert** · Web Technologies
#### 2019-03 — 2020-02
