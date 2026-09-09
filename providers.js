// Ücretsiz / kotalı yapay zeka sağlayıcıları kataloğu
// quota: uygulamanın yerel sayaçla takip ettiği tahmini limitler (resmi kaynaklara göre, değişebilir)

export const PROVIDERS = [
  {
    id: 'google',
    name: 'Google Gemini',
    color: '#4285F4',
    keyUrl: 'https://aistudio.google.com/app/apikey',
    docs: 'https://ai.google.dev/gemini-api/docs/rate-limits',
    note: 'Kredi kartı gerekmez. Kotalar Pasifik saatiyle gece yarısı sıfırlanır.',
    api: 'gemini',
    baseUrl: 'https://generativelanguage.googleapis.com/v1beta',
    models: [
      { id: 'gemini-2.5-flash', label: 'Gemini 2.5 Flash', quota: { rpm: 10, rpd: 250 }, reset: 'daily-pt' },
      { id: 'gemini-2.5-flash-lite', label: 'Gemini 2.5 Flash-Lite', quota: { rpm: 15, rpd: 1000 }, reset: 'daily-pt' },
      { id: 'gemini-2.5-pro', label: 'Gemini 2.5 Pro', quota: { rpm: 5, rpd: 100 }, reset: 'daily-pt' },
      { id: 'gemma-3-27b-it', label: 'Gemma 3 27B', quota: { rpm: 30, rpd: 14400 }, reset: 'daily-pt' }
    ]
  },
  {
    id: 'groq',
    name: 'Groq',
    color: '#F55036',
    keyUrl: 'https://console.groq.com/keys',
    docs: 'https://console.groq.com/docs/rate-limits',
    note: 'En hızlı ücretsiz çıkarım (LPU). Kart istemez.',
    api: 'openai',
    baseUrl: 'https://api.groq.com/openai/v1',
    models: [
      { id: 'llama-3.3-70b-versatile', label: 'Llama 3.3 70B', quota: { rpm: 30, rpd: 1000 }, reset: 'daily-utc' },
      { id: 'llama-3.1-8b-instant', label: 'Llama 3.1 8B Instant', quota: { rpm: 30, rpd: 14400 }, reset: 'daily-utc' },
      { id: 'openai/gpt-oss-120b', label: 'GPT-OSS 120B', quota: { rpm: 30, rpd: 1000 }, reset: 'daily-utc' },
      { id: 'openai/gpt-oss-20b', label: 'GPT-OSS 20B', quota: { rpm: 30, rpd: 1000 }, reset: 'daily-utc' },
      { id: 'qwen/qwen3-32b', label: 'Qwen3 32B', quota: { rpm: 30, rpd: 1000 }, reset: 'daily-utc' },
      { id: 'meta-llama/llama-4-scout-17b-16e-instruct', label: 'Llama 4 Scout', quota: { rpm: 15, rpd: 500 }, reset: 'daily-utc' }
    ]
  },
  {
    id: 'openrouter',
    name: 'OpenRouter',
    color: '#7C3AED',
    keyUrl: 'https://openrouter.ai/keys',
    docs: 'https://openrouter.ai/docs/api-reference/limits',
    note: 'Tek anahtarla onlarca ücretsiz model (:free). Günde 50 istek (10$ yüklersen 1000).',
    api: 'openai',
    baseUrl: 'https://openrouter.ai/api/v1',
    models: [
      { id: 'deepseek/deepseek-r1:free', label: 'DeepSeek R1 (free)', quota: { rpm: 20, rpd: 50 }, reset: 'daily-utc' },
      { id: 'deepseek/deepseek-chat-v3-0324:free', label: 'DeepSeek V3 (free)', quota: { rpm: 20, rpd: 50 }, reset: 'daily-utc' },
      { id: 'qwen/qwen3-coder:free', label: 'Qwen3 Coder (free)', quota: { rpm: 20, rpd: 50 }, reset: 'daily-utc' },
      { id: 'meta-llama/llama-4-scout:free', label: 'Llama 4 Scout (free)', quota: { rpm: 20, rpd: 50 }, reset: 'daily-utc' },
      { id: 'google/gemma-3-27b-it:free', label: 'Gemma 3 27B (free)', quota: { rpm: 20, rpd: 50 }, reset: 'daily-utc' },
      { id: 'mistralai/mistral-small-3.2-24b-instruct:free', label: 'Mistral Small 3.2 (free)', quota: { rpm: 20, rpd: 50 }, reset: 'daily-utc' }
    ]
  },
  {
    id: 'mistral',
    name: 'Mistral AI',
    color: '#FF7000',
    keyUrl: 'https://console.mistral.ai/api-keys',
    docs: 'https://docs.mistral.ai/deployment/laplateforme/tier/',
    note: 'Ücretsiz "Experiment" modu: ~1 milyar token/ay, düşük RPS.',
    api: 'openai',
    baseUrl: 'https://api.mistral.ai/v1',
    models: [
      { id: 'mistral-small-latest', label: 'Mistral Small', quota: { rpm: 60, monthlyTokens: 1000000000 }, reset: 'monthly' },
      { id: 'mistral-medium-latest', label: 'Mistral Medium', quota: { rpm: 60, monthlyTokens: 1000000000 }, reset: 'monthly' },
      { id: 'codestral-latest', label: 'Codestral', quota: { rpm: 30, monthlyTokens: 1000000000 }, reset: 'monthly' },
      { id: 'magistral-small-latest', label: 'Magistral Small (reasoning)', quota: { rpm: 30, monthlyTokens: 1000000000 }, reset: 'monthly' }
    ]
  },
  {
    id: 'cerebras',
    name: 'Cerebras',
    color: '#F26B21',
    keyUrl: 'https://cloud.cerebras.ai/',
    docs: 'https://inference-docs.cerebras.ai/support/pricing',
    note: 'Günde ~1M token, çok yüksek hız.',
    api: 'openai',
    baseUrl: 'https://api.cerebras.ai/v1',
    models: [
      { id: 'llama-3.3-70b', label: 'Llama 3.3 70B', quota: { rpm: 30, rpd: 14400, dailyTokens: 1000000 }, reset: 'daily-utc' },
      { id: 'llama3.1-8b', label: 'Llama 3.1 8B', quota: { rpm: 30, rpd: 14400, dailyTokens: 1000000 }, reset: 'daily-utc' },
      { id: 'qwen-3-32b', label: 'Qwen3 32B', quota: { rpm: 30, rpd: 14400, dailyTokens: 1000000 }, reset: 'daily-utc' },
      { id: 'gpt-oss-120b', label: 'GPT-OSS 120B', quota: { rpm: 30, rpd: 14400, dailyTokens: 1000000 }, reset: 'daily-utc' }
    ]
  },
  {
    id: 'github',
    name: 'GitHub Models',
    color: '#8B949E',
    keyUrl: 'https://github.com/settings/tokens',
    docs: 'https://docs.github.com/en/github-models/prototyping-with-ai-models',
    note: 'GitHub hesabı + "models:read" izinli Personal Access Token yeter. GPT-4o, o3, Grok gibi modeller.',
    api: 'openai',
    baseUrl: 'https://models.github.ai/inference',
    models: [
      { id: 'openai/gpt-4o', label: 'GPT-4o', quota: { rpm: 10, rpd: 50 }, reset: 'daily-utc' },
      { id: 'openai/gpt-4o-mini', label: 'GPT-4o mini', quota: { rpm: 15, rpd: 150 }, reset: 'daily-utc' },
      { id: 'openai/gpt-4.1', label: 'GPT-4.1', quota: { rpm: 10, rpd: 50 }, reset: 'daily-utc' },
      { id: 'openai/o3-mini', label: 'o3-mini', quota: { rpm: 10, rpd: 50 }, reset: 'daily-utc' },
      { id: 'meta/Llama-4-Scout-17B-16E-Instruct', label: 'Llama 4 Scout', quota: { rpm: 15, rpd: 150 }, reset: 'daily-utc' },
      { id: 'xai/grok-3-mini', label: 'Grok 3 mini', quota: { rpm: 15, rpd: 150 }, reset: 'daily-utc' }
    ]
  },
  {
    id: 'nvidia',
    name: 'NVIDIA NIM',
    color: '#76B900',
    keyUrl: 'https://build.nvidia.com/',
    docs: 'https://build.nvidia.com/',
    note: 'Kayıt olunca 1.000 kredi (iş e-postasıyla 5.000).',
    api: 'openai',
    baseUrl: 'https://integrate.api.nvidia.com/v1',
    models: [
      { id: 'deepseek-ai/deepseek-r1', label: 'DeepSeek R1', quota: { rpm: 40, credits: 1000 }, reset: 'credits' },
      { id: 'meta/llama-3.3-70b-instruct', label: 'Llama 3.3 70B', quota: { rpm: 40, credits: 1000 }, reset: 'credits' },
      { id: 'qwen/qwen3-235b-a22b', label: 'Qwen3 235B', quota: { rpm: 40, credits: 1000 }, reset: 'credits' },
      { id: 'nvidia/llama-3.1-nemotron-70b-instruct', label: 'Nemotron 70B', quota: { rpm: 40, credits: 1000 }, reset: 'credits' }
    ]
  },
  {
    id: 'cohere',
    name: 'Cohere',
    color: '#39594D',
    keyUrl: 'https://dashboard.cohere.com/api-keys',
    docs: 'https://docs.cohere.com/docs/rate-limits',
    note: 'Deneme anahtarı: ayda 1.000 çağrı, ticari kullanım yok.',
    api: 'openai',
    baseUrl: 'https://api.cohere.ai/compatibility/v1',
    models: [
      { id: 'command-a-03-2025', label: 'Command A', quota: { rpm: 20, monthly: 1000 }, reset: 'monthly' },
      { id: 'command-r-plus-08-2024', label: 'Command R+', quota: { rpm: 20, monthly: 1000 }, reset: 'monthly' },
      { id: 'command-r7b-12-2024', label: 'Command R7B', quota: { rpm: 20, monthly: 1000 }, reset: 'monthly' }
    ]
  },
  {
    id: 'deepseek',
    name: 'DeepSeek',
    color: '#4D6BFE',
    keyUrl: 'https://platform.deepseek.com/api_keys',
    docs: 'https://api-docs.deepseek.com/',
    note: 'Kayıt hediyesi token, sonrası çok ucuz kullandıkça öde.',
    api: 'openai',
    baseUrl: 'https://api.deepseek.com/v1',
    models: [
      { id: 'deepseek-chat', label: 'DeepSeek Chat (V3)', quota: { rpm: 60, credits: 5000000 }, reset: 'credits' },
      { id: 'deepseek-reasoner', label: 'DeepSeek Reasoner (R1)', quota: { rpm: 60, credits: 5000000 }, reset: 'credits' }
    ]
  },
  {
    id: 'huggingface',
    name: 'Hugging Face',
    color: '#FFD21E',
    keyUrl: 'https://huggingface.co/settings/tokens',
    docs: 'https://huggingface.co/docs/inference-providers',
    note: 'Aylık küçük ücretsiz kredi ($0.10), 200+ yönlendirilmiş model.',
    api: 'openai',
    baseUrl: 'https://router.huggingface.co/v1',
    models: [
      { id: 'openai/gpt-oss-120b', label: 'GPT-OSS 120B', quota: { rpm: 10, monthly: 300 }, reset: 'monthly' },
      { id: 'meta-llama/Llama-3.3-70B-Instruct', label: 'Llama 3.3 70B', quota: { rpm: 10, monthly: 300 }, reset: 'monthly' },
      { id: 'Qwen/Qwen3-235B-A22B-Instruct-2507', label: 'Qwen3 235B', quota: { rpm: 10, monthly: 300 }, reset: 'monthly' }
    ]
  },
  {
    id: 'sambanova',
    name: 'SambaNova',
    color: '#EE7624',
    keyUrl: 'https://cloud.sambanova.ai/apis',
    docs: 'https://docs.sambanova.ai/',
    note: 'Ücretsiz geliştirici katmanı + deneme kredisi.',
    api: 'openai',
    baseUrl: 'https://api.sambanova.ai/v1',
    models: [
      { id: 'Meta-Llama-3.3-70B-Instruct', label: 'Llama 3.3 70B', quota: { rpm: 20, rpd: 1000 }, reset: 'daily-utc' },
      { id: 'DeepSeek-R1-Distill-Llama-70B', label: 'DeepSeek R1 Distill 70B', quota: { rpm: 10, rpd: 500 }, reset: 'daily-utc' }
    ]
  },
  {
    id: 'openai',
    name: 'OpenAI (kendi anahtarın)',
    color: '#10A37F',
    keyUrl: 'https://platform.openai.com/api-keys',
    docs: 'https://platform.openai.com/docs',
    note: 'Ücretsiz değil; deneme kredin veya kendi bakiyen varsa kullan.',
    api: 'openai',
    baseUrl: 'https://api.openai.com/v1',
    models: [
      { id: 'gpt-4o-mini', label: 'GPT-4o mini', quota: { rpm: 60 }, reset: 'none' },
      { id: 'gpt-4o', label: 'GPT-4o', quota: { rpm: 60 }, reset: 'none' }
    ]
  }
];

export function findModel(providerId, modelId) {
  const p = PROVIDERS.find((x) => x.id === providerId);
  if (!p) return null;
  const m = p.models.find((x) => x.id === modelId);
  if (!m) return null;
  return { provider: p, model: m };
}
