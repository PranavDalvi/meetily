### Task: Implement Speaking Language Selection & Fast Hindi/English Auto-Detection in Tauri Note-Taking App
  
    #### 1. Background & Problem
    Our desktop app records/captures audio and sends chunks to an OpenAI-compatible ASR endpoint (`indic-transcribe-flex` via LiteLLM or directly).
    Currently:
    - There is no UI switch for speaking language.
    - When audio is transcribed without a language parameter, the model performs unconstrained auto-detection across 25 Indian languages. As a result, Hindi speech frequently gets absorbed by Urdu (`ur` which emits Perso-Arabic script)
    or regional dialects (`hne`, `mr`, `bho`), and English speech can be transliterated incorrectly.
  
    #### 2. Goal
    1. Add a **Speaking Language Switch** in the Tauri UI.
    2. In the transcription handler (Rust backend `tauri::command` or frontend fetch), implement a **smart, low-latency language routing strategy**:
       - If **Hindi** is selected: force `language="hi"` (Devanagari script).
       - If **English** is selected: force `language="en"` (Latin script).
       - If **Auto (Hindi / English)** is selected: automatically detect between Hindi and English with minimal latency (<250ms), and prevent Arabic/Urdu or dialect scripts from appearing in the notes.
  
    ---
  
    #### 3. Required Changes
  
    ### A. Frontend UI (Settings or Control Bar)
    Add a language selector dropdown:
    - Options:
      1. `Auto (Hindi / English)` — *default* (`value: "auto_hi_en"`)
      2. `Hindi (हिंदी)` (`value: "hi"`)                                                                                                                                                                                                      
      3. `English` (`value: "en"`)                                                                                                                                                                                                         
      4. `All Languages (Auto)` (`value: "all"`)
    - Persist the user's choice across app restarts using `localStorage` or `tauri-plugin-store`.
    - Pass the selected language value whenever dispatching audio transcription.
  
    ---
  
    ### B. Transcription Client Logic (Rust `tauri::command` or JS/TS API call)
    Update the audio transcription request:                                                                                                                                                                                                
                                                                                                                                                                                                                                           
    1. **For Explicit Modes (`hi` or `en`):**                                                                                                                                                                                              
       - Add `language="hi"` or `language="en"` to the `multipart/form-data` payload.
       - Optional: Add `mode="mixed"` so code-mixed Hinglish preserves English terms in Latin script and Hindi in Devanagari.
       - This takes only 1 single pass with **0 ms latency overhead**.
  
    2. **For `Auto (Hindi / English)` (`auto_hi_en`):**
       - **Step 1:** Call the transcription endpoint without the `language` parameter.
       - **Step 2:** Inspect the returned JSON:
         - Read `response.text` and `response.language` (returned by Indic-Transcribe).
       - **Step 3:**
         - If `response.language == "en"`: Return text as English.
         - If `response.language == "hi"`: Return text as Hindi.
         - If `response.language == "ur"` OR the text contains Arabic script (`\u0600` to `\u06FF`) OR non-Devanagari Indic scripts (like Punjabi/Bengali):
           - Immediately re-transcribe this single chunk with `language="hi"`.
           - Return the resulting Devanagari Hindi text.
         *(This recovery fallback only runs when an Urdu/regional sink is triggered, keeping latency minimal for >90% of chunks).*                                                                                                         
                                                                                                                                                                                                                                           
    ---                                                                                                                                                                                                                                    
                                                                                                                                                                                                                                           
    ### C. Reference Implementation Snippet (JavaScript / TypeScript in Tauri)                                                                                                                                                             
                                                                                                                                                                                                                                           
    ```typescript                                                                                                                                                                                                                          
    async function transcribeAudioChunk(                                                                                                                                                                                                   
      audioBlob: Blob,                                                                                                                                                                                                                     
      languageMode: "auto_hi_en" | "hi" | "en" | "all" = "auto_hi_en"                                                                                                                                                                      
    ): Promise<string> {                                                                                                                                                                                                                   
      const ASR_ENDPOINT = "http://localhost:4000/v1/audio/transcriptions"; // LiteLLM or ASR host                                                                                                                                         
      const API_KEY = "sk-your-key";                                                                                                                                                                                                       
                                                                                                                                                                                                                                           
      const createFormData = (lang?: string) => {                                                                                                                                                                                          
        const formData = new FormData();                                                                                                                                                                                                   
        formData.append("file", audioBlob, "chunk.wav");                                                                                                                                                                                   
        formData.append("model", "indic-transcribe-flex");                                                                                                                                                                                 
        formData.append("mode", "mixed"); // Retains English words in Latin script for Hinglish                                                                                                                                            
        if (lang) {                                                                                                                                                                                                                        
          formData.append("language", lang);                                                                                                                                                                                               
        }                                                                                                                                                                                                                                  
        return formData;                                                                                                                                                                                                                   
      };                                                                                                                                                                                                                                   
                                                                                                                                                                                                                                           
      const sendRequest = async (lang?: string) => {                                                                                                                                                                                       
        const res = await fetch(ASR_ENDPOINT, {                                                                                                                                                                                            
          method: "POST",
          headers: { Authorization: `Bearer ${API_KEY}` },
          body: createFormData(lang),
        });
        if (!res.ok) throw new Error(`ASR Error: ${res.statusText}`);
        return await res.json(); // { text: string, language?: string }
      };
  
      // 1. Explicit language selected
      if (languageMode === "hi" || languageMode === "en") {
        const data = await sendRequest(languageMode);
        return data.text?.trim() || "";
      }
  
      // 2. Unconstrained mode
      if (languageMode === "all") {
        const data = await sendRequest();
        return data.text?.trim() || "";
      }
  
      // 3. Auto (Hindi / English) mode
      const initial = await sendRequest();
      const text = initial.text?.trim() || "";
      const detected = (initial.language || "").toLowerCase();
  
      if (detected === "en" || detected === "hi") {
        return text;
      }
  
      // Check if speech was mistakenly mapped to Urdu (Arabic script) or a non-Devanagari Indic script
      const hasArabicScript = /[\u0600-\u06FF]/.test(text);
      const isIndicSink = ["ur", "pa", "bn", "gu", "ta", "te", "kn", "ml"].includes(detected);
  
      if (hasArabicScript || isIndicSink) {
        // Re-transcribe chunk with forced Hindi (Devanagari)
        const hiFallback = await sendRequest("hi");
        return hiFallback.text?.trim() || text;
      }
  
      return text;
    }
  ──────
  #### 4. Acceptance Criteria
  
  1. The user can switch between Auto (Hindi/English), Hindi, and English in the desktop UI.
  2. English speech produces clean Latin script without random Indian transliteration.
  3. Hindi speech produces clean Devanagari script, never Arabic/Urdu script.
  4. Latency remains ~200-300ms per 5s chunk.
  5. No changes are required to the shared backend model server.

