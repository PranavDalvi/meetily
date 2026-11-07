import React, { useEffect, useState } from 'react';
import { Key, Eye, EyeOff, Server, AlertCircle, Loader2, Sparkles, Mic, ChevronDown, ChevronUp, CheckCircle2 } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { OnboardingContainer } from '../OnboardingContainer';
import { useOnboarding } from '@/contexts/OnboardingContext';

export function SetupOverviewStep() {
  const { goNext, apiKey, setApiKey, endpoint, setEndpoint, validateApiKey } = useOnboarding();
  const [isMac, setIsMac] = useState(false);
  const [showApiKey, setShowApiKey] = useState(false);
  const [showAdvanced, setShowAdvanced] = useState(false);
  const [isValidating, setIsValidating] = useState(false);
  const [errorMessage, setErrorMessage] = useState<string | null>(null);

  useEffect(() => {
    const checkPlatform = async () => {
      try {
        const { platform } = await import('@tauri-apps/plugin-os');
        setIsMac(platform() === 'macos');
      } catch (e) {
        setIsMac(navigator.userAgent.includes('Mac'));
      }
    };
    checkPlatform();
  }, []);

  const handleContinue = async () => {
    const trimmed = apiKey.trim();
    if (!trimmed) {
      setErrorMessage('Please enter the access key provided by your administrator.');
      return;
    }

    setErrorMessage(null);
    setIsValidating(true);

    try {
      const result = await validateApiKey(trimmed);
      if (result.valid) {
        goNext();
      } else {
        setErrorMessage(result.error || 'Invalid access key. Please verify the key provided to you.');
      }
    } catch (e) {
      // If validation call fails unexpectedly, proceed to avoid hard blocking
      console.warn('[SetupOverviewStep] Validation error:', e);
      goNext();
    } finally {
      setIsValidating(false);
    }
  };

  return (
    <OnboardingContainer
      title="Connect Meetily AI"
      description="Enter the access key provided to you to connect to transcription and summarization services."
      step={2}
      totalSteps={isMac ? 4 : 3}
    >
      <div className="flex flex-col items-center space-y-6 max-w-lg mx-auto">
        {/* Access Key Input Card */}
        <div className="w-full bg-white rounded-xl border border-gray-200 p-6 shadow-sm space-y-4">
          <div>
            <Label htmlFor="apiKey" className="block text-sm font-semibold text-gray-900 mb-1.5 flex items-center gap-2">
              <Key className="w-4 h-4 text-gray-600" />
              Access Key (API Key)
            </Label>
            <p className="text-xs text-gray-500 mb-3">
              Provided by your administrator to activate Meetily features.
            </p>
            <div className="relative">
              <Input
                id="apiKey"
                type={showApiKey ? 'text' : 'password'}
                value={apiKey}
                onChange={(e) => {
                  setApiKey(e.target.value);
                  if (errorMessage) setErrorMessage(null);
                }}
                onKeyDown={(e) => {
                  if (e.key === 'Enter' && apiKey.trim() && !isValidating) {
                    handleContinue();
                  }
                }}
                placeholder="sk-..."
                className="pr-10 font-mono text-sm focus:ring-1 focus:ring-blue-500"
                autoFocus
              />
              <button
                type="button"
                onClick={() => setShowApiKey(!showApiKey)}
                className="absolute right-2.5 top-1/2 -translate-y-1/2 text-gray-400 hover:text-gray-600 focus:outline-none"
              >
                {showApiKey ? <EyeOff className="w-4 h-4" /> : <Eye className="w-4 h-4" />}
              </button>
            </div>
            {errorMessage && (
              <div className="flex items-center gap-2 mt-2 text-xs text-red-600">
                <AlertCircle className="w-4 h-4 flex-shrink-0" />
                <span>{errorMessage}</span>
              </div>
            )}
          </div>

          {/* Connected Services Preview */}
          <div className="pt-3 border-t border-gray-100 space-y-2">
            <span className="text-xs font-medium text-gray-600 uppercase tracking-wider">Services Included</span>
            <div className="grid grid-cols-2 gap-2 text-xs">
              <div className="flex items-center gap-2 p-2 rounded-lg bg-gray-50 border border-gray-100 text-gray-700">
                <Mic className="w-4 h-4 text-blue-600 flex-shrink-0" />
                <div>
                  <p className="font-semibold">Speech-to-Text</p>
                  <p className="text-[10px] text-gray-500">indic-transcribe</p>
                </div>
              </div>
              <div className="flex items-center gap-2 p-2 rounded-lg bg-gray-50 border border-gray-100 text-gray-700">
                <Sparkles className="w-4 h-4 text-purple-600 flex-shrink-0" />
                <div>
                  <p className="font-semibold">Meeting Summary</p>
                  <p className="text-[10px] text-gray-500">Gemma 4</p>
                </div>
              </div>
            </div>
          </div>

          {/* Advanced Settings Toggle */}
          <div className="pt-2">
            <button
              type="button"
              onClick={() => setShowAdvanced(!showAdvanced)}
              className="flex items-center gap-1.5 text-xs text-gray-500 hover:text-gray-800 transition-colors"
            >
              <Server className="w-3.5 h-3.5" />
              <span>Advanced Server Settings</span>
              {showAdvanced ? <ChevronUp className="w-3 h-3" /> : <ChevronDown className="w-3 h-3" />}
            </button>

            {showAdvanced && (
              <div className="mt-3 p-3 bg-gray-50 rounded-lg border border-gray-200 space-y-2">
                <Label htmlFor="endpoint" className="block text-xs font-medium text-gray-700">
                  Server Endpoint URL
                </Label>
                <Input
                  id="endpoint"
                  type="text"
                  value={endpoint}
                  onChange={(e) => setEndpoint(e.target.value)}
                  placeholder="https://plain-turtles-refuse.loca.lt/v1"
                  className="text-xs font-mono bg-white"
                />
                <p className="text-[10px] text-gray-500">
                  Default: https://plain-turtles-refuse.loca.lt/v1
                </p>
              </div>
            )}
          </div>
        </div>

        {/* CTA Button */}
        <div className="w-full max-w-xs space-y-3">
          <Button
            onClick={handleContinue}
            disabled={!apiKey.trim() || isValidating}
            className="w-full h-11 bg-gray-900 hover:bg-gray-800 text-white disabled:opacity-50"
          >
            {isValidating ? (
              <>
                <Loader2 className="w-4 h-4 mr-2 animate-spin" />
                Connecting...
              </>
            ) : (
              'Connect & Continue'
            )}
          </Button>

          <p className="text-center text-xs text-gray-500">
            Don't have a key? Contact your Meetily administrator.
          </p>
        </div>
      </div>
    </OnboardingContainer>
  );
}
