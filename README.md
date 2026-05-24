# authentiq-pdf-service

Microservice Rust (Axum) pour rasteriser des PDF lourds via **Poppler** (`pdftoppm`), déployable sur **Fly.io**.  
Laravel (`authentiq-laravel`) l'appelle en HTTP lorsque le traitement navigateur (pdf.js) devient trop lent.

## Endpoints

| Méthode | Route | Description |
|---------|-------|-------------|
| `GET` | `/health` | Santé (sans auth) |
| `POST` | `/v1/rasterize?dpi=150&max_pages=100` | PDF → pages JPEG (base64) |

### Authentification

Header obligatoire :

```http
X-Api-Key: votre-clé-secrete
```

### Requête

`multipart/form-data` avec champ **`file`** (PDF).

### Réponse (200)

```json
{
  "status": "success",
  "page_count": 2,
  "pages": [
    {
      "page_number": 1,
      "width": 1240,
      "height": 1754,
      "mime": "image/jpeg",
      "data_base64": "..."
    }
  ],
  "meta": { "dpi": 150, "processing_ms": 842 }
}
```

## Variables d'environnement

| Variable | Défaut | Description |
|----------|--------|-------------|
| `PORT` | `8080` | Port HTTP |
| `PDF_SERVICE_API_KEY` | — | Clé partagée avec Laravel |
| `PDF_MAX_UPLOAD_MB` | `50` | Taille max upload |
| `PDF_MAX_PAGES` | `100` | Pages max extraites |
| `PDF_DEFAULT_DPI` | `150` | DPI si non précisé |
| `PDFTOPPM_BIN` | `pdftoppm` | Binaire Poppler |

## Développement local

Prérequis : Rust 1.83+, Poppler (`brew install poppler` sur macOS).

```bash
export PDF_SERVICE_API_KEY=dev-secret
cargo run
```

Test :

```bash
curl -s -X POST "http://localhost:8080/v1/rasterize?dpi=150" \
  -H "X-Api-Key: dev-secret" \
  -F "file=@/chemin/vers/document.pdf" | jq '.page_count, .meta'
```

## Déploiement Fly.io

```bash
cd authentiq-pdf-service
fly launch --no-deploy   # si première fois
fly secrets set PDF_SERVICE_API_KEY="$(openssl rand -hex 32)"
fly deploy
fly status
```

Notez l'URL : `https://authentiq-pdf-service.fly.dev`

## Intégration Laravel

Dans `authentiq-laravel/.env` :

```env
AUTHENTIQ_PDF_SERVICE_ENABLED=true
AUTHENTIQ_PDF_SERVICE_URL=https://authentiq-pdf-service.fly.dev
AUTHENTIQ_PDF_SERVICE_API_KEY=la-même-clé-que-fly-secrets
AUTHENTIQ_PDF_SERVICE_MIN_BYTES=2097152
```

- Fichiers **≥ 2 Mo** (configurable) : rasterisation côté microservice via `POST /api/encodage/rasterize-pdf`
- Fichiers légers : pdf.js dans le navigateur (comportement actuel)

## Évolutions possibles

- Job async + stockage S3 (éviter base64 pour gros volumes)
- Webhook de fin de traitement
- Queue Redis entre Laravel et le worker Rust
- Remplacement pdftoppm par PDFium natif (binaire unique)

## Repo

Projet **séparé** de `authentiq-laravel`. Initialiser git ici :

```bash
git init
git add .
git commit -m "feat: microservice PDF Rust pour Authentiq"
```
