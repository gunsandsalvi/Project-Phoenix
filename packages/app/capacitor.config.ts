import type { CapacitorConfig } from '@capacitor/cli';

const config: CapacitorConfig = {
  appId: 'org.phoenix.model',
  appName: 'Phoenix',
  webDir: 'dist',
  android: {
    allowMixedContent: false,
  },
};

export default config;
