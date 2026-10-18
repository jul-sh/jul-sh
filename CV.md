# Juliette Pluto

_Adversarial robustness for frontier AI systems at Google DeepMind. Previously: trusted execution environments, privacy-preserving ML, and web engineering._

[j@jul.sh](mailto:j@jul.sh)

## Experience

### **Google DeepMind** · Staff Software Engineer
#### 2020-04 — present · NYC · promoted to Senior 2024, Staff 2026

* Started DeepMind’s first effort to defend against prompt injection. Built the security evaluation suite used to select frontier Gemini models, and extended adversarial evals to cover production agents across Workspace, Cloud, and Search.
* Developed a new training approach for adaptive prompt injection robustness on Gemini, pitching it past initial skepticism to get it staffed. With the same data budget, it improved robustness 5x on our automated red-teaming benchmark.
* Brought in to close a blocking prompt injection gap for a multi-billion-dollar enterprise contract. Dug into the model’s failure modes and reworked the training mix, doubling robustness on our internal benchmark, and worked directly with the customer’s Head of ML Safety to get the deployment signed off.
* Built the *User Instruction Classifier*, an early policy-based defense that hit 99.97% robustness against adaptive attacks on our internal benchmark. Saturating the benchmark showed where the eval fell short of the real threat model and pushed our focus toward harder attack classes; the classifier itself was later adopted in production safety systems to harden against human red-teaming.
* Found new prompt injection attack surfaces in Google Search, led a four-engineer effort across teams to fix them, and co-wrote parts of Google’s internal Prompt Injection Playbook.
* Earlier at Google (2020–2023, in London): worked on trusted execution environments and privacy-preserving ML. Designed a TEE orchestrator to shrink the trusted computing base for remotely attested ML workloads (including Gboard), built attested end-to-end encryption from the browser to the enclave, and caught critical vulnerabilities during infrastructure design.

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
* *Beyond the Paper: Operationalizing Auto Red Teaming for Measurable Model Evaluation & Development.* AI Red Team Colloquium.

## Education

### **Humboldt University**
#### 2013-10 — 2016-03 · Berlin

## Recognition

### **Google Developer Expert** · Web Technologies
#### 2019-03 — 2020-02
