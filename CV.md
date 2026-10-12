# Juliette Pluto

_Adversarial robustness, ML security, and confidential computing._

## Experience

### **Google DeepMind** · Staff Software Engineer
#### 2020-04 — present · NYC & London · promoted to Senior 2024, Staff 2026

* Started DeepMind's first effort to defend against prompt injection. Built the security evaluation suite used in frontier model selection and expanded adversarial eval coverage across Workspace, Cloud, and Search.
* Proposed and led a new training approach for adaptive prompt injection robustness in Gemini, improving performance 5x on our internal automated red-teaming benchmark on the same data budget.
* Led the work to close a prompt injection gap blocking a major enterprise deployment. Tracked down the failure modes in our training data, doubled robustness on the target benchmark, and worked with the customer's safety team through sign-off.
* Built the User Instruction Classifier, an early policy-based defense that reached 99.97% robustness against adaptive attacks on our internal benchmark. Saturating the benchmark exposed where our evals fell short of the real threat model and shifted our training work toward harder attack classes; the classifier itself was later adopted in production safety systems to defend against human red-teaming.
* Found unaddressed prompt injection surfaces in Google Search, led a four-engineer cross-org effort to fix them, and co-authored parts of Google's internal Prompt Injection Playbook.
* Before 2024, worked on trusted execution environments and privacy-preserving ML, designing a TEE orchestrator to shrink the trusted computing base for remotely attested ML workloads (such as Gboard) and building browser-to-enclave end-to-end encryption for private inference.

### **N26** · Software Engineer
#### 2018-01 — 2020-03 · Berlin

Joined a four-person team rebuilding N26's web platform from scratch and helped grow the team past 20 engineers. Led full-stack product work, mentored engineers, and focused on production reliability and web performance (30% fewer production errors, 20% faster first meaningful paint, 1.4s faster time-to-interactive).

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
