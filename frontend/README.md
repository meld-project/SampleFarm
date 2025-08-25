# SampleFarm Frontend

A modern React-based web interface for the SampleFarm malware analysis platform, built with Next.js and TypeScript.

## Features

- **Sample Management**: Upload, browse, and manage malware samples with advanced filtering
- **Task Management**: Create and monitor analysis tasks with real-time progress tracking
- **Sandbox Integration**: Manage CAPE and CFG sandbox instances through intuitive interfaces
- **Analysis Results**: View detailed analysis reports with interactive visualizations
- **File Operations**: Secure file upload with drag-and-drop support and batch operations
- **Responsive Design**: Modern UI built with Tailwind CSS and shadcn/ui components
- **Real-time Updates**: Live status updates using React Query for optimal UX

## Tech Stack

### Core Framework
- **Next.js 15** - React framework with App Router
- **React 19** - UI library with latest features
- **TypeScript** - Type-safe development

### UI & Styling
- **Tailwind CSS** - Utility-first CSS framework
- **shadcn/ui** - High-quality React components built on Radix UI
- **Lucide React** - Beautiful icon library
- **Recharts** - Composable charting library

### State Management & Data Fetching
- **Zustand** - Lightweight state management
- **TanStack Query** - Powerful data synchronization
- **React Hook Form** - Performant forms with validation
- **Zod** - TypeScript-first schema validation

### Development Tools
- **ESLint** - Code linting and formatting
- **Turbopack** - Fast bundler for development

## Project Structure

```
frontend/
├── src/
│   ├── app/                    # Next.js App Router pages
│   │   ├── files/             # Sample file management
│   │   ├── tasks/             # Task management
│   │   ├── cape-management/   # CAPE sandbox management
│   │   └── cfg-management/    # CFG sandbox management
│   ├── components/            # Reusable UI components
│   │   ├── ui/               # Base UI components (shadcn/ui)
│   │   ├── task-management/  # Task-related components
│   │   ├── cape-management/  # CAPE-related components
│   │   └── cfg-management/   # CFG-related components
│   └── lib/                  # Utilities and configurations
│       ├── api.ts           # API client configuration
│       ├── config.ts        # Runtime configuration
│       ├── types.ts         # TypeScript type definitions
│       └── utils.ts         # Utility functions
├── public/                   # Static assets
│   └── config.json          # Runtime configuration
├── config.json              # Development configuration
└── env.example              # Environment variables template
```

## Configuration

### Environment Setup

1. Copy the environment template:
   ```bash
   cp env.example .env.local
   ```

2. Configure your environment variables:
   ```bash
   # Backend API URL
   NEXT_PUBLIC_API_URL=http://localhost:8080
   
   # Optional: Custom configuration
   NEXT_PUBLIC_CONFIG_URL=/config.json
   ```

### Runtime Configuration

The application uses a flexible configuration system with the following priority:

```
Environment Variables > config.json > Default Configuration
```

Edit `public/config.json` for runtime configuration:

```json
{
  "backend": {
    "url": "http://localhost:8080",
    "timeout": 30000,
    "retries": 3
  },
  "app": {
    "title": "SampleFarm - Sample Management System",
    "description": "Professional malware sample management and analysis platform",
    "version": "1.0.0"
  },
  "ui": {
    "theme": "light",
    "pageSize": 20,
    "maxFileSize": "100MB"
  }
}
```

## Development

### Prerequisites
- Node.js 18+ 
- npm, yarn, or pnpm

### Setup

1. Install dependencies:
   ```bash
   npm install
   # or
   yarn install
   # or
   pnpm install
   ```

2. Start the development server:
   ```bash
   npm run dev
   # or
   yarn dev
   # or
   pnpm dev
   ```

3. Open [http://localhost:3000](http://localhost:3000) in your browser.

### Available Scripts

- `npm run dev` - Start development server with Turbopack
- `npm run build` - Build for production
- `npm run start` - Start production server
- `npm run lint` - Run ESLint

### Code Quality

The project includes ESLint configuration for code quality:

```bash
# Run linter
npm run lint

# Auto-fix issues
npm run lint -- --fix
```

## Building for Production

1. Build the application:
   ```bash
   npm run build
   ```

2. Start the production server:
   ```bash
   npm run start
   ```

## Deployment

### Docker Deployment

The frontend can be deployed using Docker. See the main project's `docker-compose.yml` for configuration.

### Static Export

For static hosting, configure Next.js for static export in `next.config.ts`:

```typescript
/** @type {import('next').NextConfig} */
const nextConfig = {
  output: 'export',
  trailingSlash: true,
  images: {
    unoptimized: true
  }
}
```

### Environment Variables for Production

Set the following environment variables for production:

- `NEXT_PUBLIC_API_URL` - Backend API URL
- `NODE_ENV=production`

## API Integration

The frontend communicates with the SampleFarm backend through a REST API. Key endpoints include:

- **Samples**: `/api/samples/*` - File upload and management
- **Tasks**: `/api/tasks/*` - Analysis task operations
- **CAPE**: `/api/cape-instances/*` - CAPE sandbox management
- **CFG**: `/api/cfg-instances/*` - CFG sandbox management

API client configuration is located in `src/lib/api.ts`.

## Contributing

1. Follow the existing code style and patterns
2. Use TypeScript for type safety
3. Write meaningful component and function names
4. Add proper error handling and loading states
5. Test your changes thoroughly

## Browser Support

- Chrome/Chromium 90+
- Firefox 88+
- Safari 14+
- Edge 90+

## License

See the main project LICENSE file for details.