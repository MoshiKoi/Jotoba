/**
 * This JS-File implements the Speech to Text functionality for text input
 */

// Constructor preparing the recognition
function SpeechOverlay() {
    this.firstToggle = true;

    try {
        let SpeechRecognition = SpeechRecognition || webkitSpeechRecognition;
        this.recognition = new SpeechRecognition();
    } catch (e) {}
};

// Handles the initial setup of the recognition lib 
SpeechOverlay.recognitionSetup = () => {
    this.recognition.lang = 'en-US';
    this.recognition.continuous = false;
    this.recognition.interimResults = false;
    this.recognition.maxAlternatives = 1;
    
    // On recognition start
    this.recognition.onstart = function() {
        $('#currentlyListening').html(getText("SPEECH_LISTEN_YES"));
        $('.voiceSvg').toggleClass("active");
    };
    
    // On recognition error
    this.recognition.onerror  = function(event) { 
        switch(event.error) {
            case "not-allowed":
                Util.showMessage("error", getText("SPEECH_NO_PERMISSION"));
                break;
            case "aborted":
                Util.showMessage("info", getText("SPEECH_ABORT"));
                break;
            case "no-speech":
                Util.showMessage("info", getText("SPEECH_NO_VOICE"));
                break;
            default:
                Util.showMessage("error", getText("SPEECH_NOT_SUPPORTED"));
        }
        $('#currentlyListening').html(getText("SPEECH_LISTEN_NO"));
        $('.voiceSvg').toggleClass("active");
    }
    
    // On speech end
    this.recognition.onspeechend = function() {
        this.recognition.stop();
        $('#currentlyListening').html(getText("SPEECH_LISTEN_NO"));
        $('.voiceSvg').toggleClass("active");
    }
    
    // On recognition result
    this.recognition.onresult = function(event) {
        let transcript = event.results[0][0].transcript;
        $('#search').val(transcript);
    };
}

// Toggles the overlay on and off
SpeechOverlay.toggle = () => {
    if (this.recognition == undefined) {
        Util.showMessage("error", getText("SPEECH_NOT_SUPPORTED"));
        return;
    }

    closeAllSubSearchbarOverlays("speech");

    let overlay = $('.overlay.speech');
    overlay.toggleClass('hidden');

    if (overlay.hasClass("hidden")) {
        this.recognition.abort();
        this.recognition.stop();
    } else if (this.firstToggle) {
        this.firstToggle = false;
        this.recognitionSetup();
    }
}

// Activate the given language for speech recognition TODO save in cookie
SpeechOverlay.setRecognitionLang = (lang) => {
    this.recognition.abort();

    switch(lang) {
        case "jap":
            this.recognition.lang = "ja";
            $('#currentSpeechLang').html(getText("LANG_JAP"));
            break
        case "ger":
            this.recognition.lang = "de-DE";
            $('#currentSpeechLang').html(getText("LANG_GER"));
            break
        case "eng":
            this.recognition.lang = "en-US";
            $('#currentSpeechLang').html(getText("LANG_ENG"));
            break
        case "rus":
            this.recognition.lang = "ru";
            $('#currentSpeechLang').html(getText("LANG_RUS"));
            break
        case "spa":
            this.recognition.lang = "es-ES";
            $('#currentSpeechLang').html(getText("LANG_SPA"));
            break
        case "swe":
            this.recognition.lang = "sv-SE";
            $('#currentSpeechLang').html(getText("LANG_SWE"));
            break
        case "fre":
            this.recognition.lang = "fr-FR";
            $('#currentSpeechLang').html(getText("LANG_FRE"));
            break
        case "dut":
            this.recognition.lang = "nl-NL";
            $('#currentSpeechLang').html(getText("LANG_DUT"));
            break
        case "hun":
            this.recognition.lang = "hu";
            $('#currentSpeechLang').html(getText("LANG_HUN"));
            break
        case "slv":
            this.recognition.lang = "sl-SI";
            $('#currentSpeechLang').html(getText("LANG_SLV"));
            break
    }

    setTimeout(function(){ this.recognition.start(); }, 400);
}