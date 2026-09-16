const apiKey = process.env.GEMINI_API_KEY; // I'll pass this in
const baseUrl = 'https://generativelanguage.googleapis.com/v1beta/openai/v1/models';

fetch(baseUrl, {
  headers: { 'Authorization': `Bearer ${apiKey}` }
})
.then(res => res.json())
.then(data => console.log(JSON.stringify(data, null, 2)))
.catch(err => console.error(err));
