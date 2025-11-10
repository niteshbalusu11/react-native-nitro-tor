import { useEffect, useState } from 'react';
import {
  Text,
  View,
  StyleSheet,
  Button,
  ScrollView,
  Platform,
  SafeAreaView,
} from 'react-native';
import {
  DocumentDirectoryPath,
  exists,
  mkdir,
} from '@dr.pogodin/react-native-fs';
import { RnTor } from 'react-native-nitro-tor';

// Constants
const TOR_DATA_PATH = `${DocumentDirectoryPath}/tor_data`;

interface TorState {
  isSuccess: boolean | undefined;
  errorMessage: string | undefined;
  onionUrl: string | undefined;
  controlUrl: string | undefined;
}

interface RequestResult {
  status: number;
  body: string;
  error: string;
}

export default function TorApp() {
  const [torState, setTorState] = useState<TorState>({
    isSuccess: undefined,
    errorMessage: undefined,
    onionUrl: undefined,
    controlUrl: undefined,
  });
  const [getResult, setGetResult] = useState<RequestResult | null>(null);
  const [postResult, setPostResult] = useState<RequestResult | null>(null);
  const [putResult, setPutResult] = useState<RequestResult | null>(null);
  const [deleteResult, setDeleteResult] = useState<RequestResult | null>(null);

  const clearAllResults = () => {
    setGetResult(null);
    setPostResult(null);
    setPutResult(null);
    setDeleteResult(null);
  };

  useEffect(() => {
    const initTor = async () => {
      try {
        // Ensure directory exists
        await ensureDataDirectory();

        console.log('Ensuring data directory');

        // Initialize service
        const result = await RnTor.startTorIfNotRunning({
          data_dir: TOR_DATA_PATH,
          socks_port: 9050,
          target_port: 9051,
          timeout_ms: 60000,
        });

        // Log initialization result
        console.log(result);

        if (!result.is_success) {
          throw new Error('Failed to initialize Tor service');
        }

        console.log('Tor service status', await RnTor.getServiceStatus());

        setTorState({
          isSuccess: result.is_success,
          errorMessage: result.error_message,
          onionUrl: result.onion_address,
          controlUrl: result.control,
        });
      } catch (error: any) {
        console.error('Error in Tor initialization:', error);
        setTorState(prev => ({
          ...prev,
          errorMessage: error.message,
          isSuccess: false,
        }));
      }
    };

    initTor();

    // Cleanup on unmount
    return () => {
      RnTor.shutdownService().catch(console.error);
    };
  }, []);

  const ensureDataDirectory = async () => {
    try {
      const dirExists = await exists(TOR_DATA_PATH);
      if (!dirExists) {
        await mkdir(TOR_DATA_PATH, {
          NSURLIsExcludedFromBackupKey: true, // iOS specific
        });
      }
    } catch (error: any) {
      console.error('Error with directory setup:', error);
      throw new Error(`Failed to setup data directory: ${error.message}`);
    }
  };

  const httpGet = async () => {
    try {
      const result = await RnTor.httpGet({
        headers: '',
        timeout_ms: 20000,
        url: 'https://httpbin.org/get',
      });
      console.log('httpGet result', result);
      setGetResult({
        status: result.status_code,
        body: result.body,
        error: result.error,
      });
    } catch (err: any) {
      console.error('httpGet error', err);
      setGetResult({
        status: 0,
        body: '',
        error: err.message,
      });
    }
  };

  const httpPost = async () => {
    try {
      const result = await RnTor.httpPost({
        url: 'http://httpbin.org/post',
        body: '{"test":"data"}',
        headers: '{"Content-Type":"application/json"}',
        timeout_ms: 20000,
      });
      console.log('http post result', result);
      setPostResult({
        status: result.status_code,
        body: result.body,
        error: result.error,
      });
    } catch (err: any) {
      console.error('httpPost error', err);
      setPostResult({
        status: 0,
        body: '',
        error: err.message,
      });
    }
  };

  const httpPut = async () => {
    try {
      const result = await RnTor.httpPut({
        url: 'http://httpbin.org/put',
        body: '{"updated":"value"}',
        headers: '{"Content-Type":"application/json"}',
        timeout_ms: 20000,
      });
      console.log('http put result', result);
      setPutResult({
        status: result.status_code,
        body: result.body,
        error: result.error,
      });
    } catch (err: any) {
      console.error('httpPut error', err);
      setPutResult({
        status: 0,
        body: '',
        error: err.message,
      });
    }
  };

  const httpDelete = async () => {
    try {
      const result = await RnTor.httpDelete({
        url: 'http://httpbin.org/delete',
        headers: '{"Content-Type":"application/json"}',
        timeout_ms: 20000,
      });
      console.log('http delete result', result);
      setDeleteResult({
        status: result.status_code,
        body: result.body,
        error: result.error,
      });
    } catch (err: any) {
      console.error('httpDelete error', err);
      setDeleteResult({
        status: 0,
        body: '',
        error: err.message,
      });
    }
  };

  const renderResult = (title: string, result: RequestResult | null) => {
    if (!result) return null;

    return (
      <View style={styles.resultContainer}>
        <Text style={styles.resultTitle}>{title}</Text>
        <Text style={styles.resultText}>Status: {result.status}</Text>
        {result.error ? (
          <Text style={styles.errorText}>Error: {result.error}</Text>
        ) : (
          <ScrollView style={styles.responseBody}>
            <Text>{result.body}</Text>
          </ScrollView>
        )}
      </View>
    );
  };

  return (
    <SafeAreaView style={styles.scrollContainer}>
      <ScrollView>
        <View style={styles.container}>
          <Text style={styles.headerText}>Tor Network Status</Text>

          <Text style={styles.statusText}>
            Init Tor Result: {String(torState.isSuccess)}
          </Text>
          <Text style={styles.statusText}>
            Onion URL: {torState.onionUrl || 'Not available'}
          </Text>
          <Text style={styles.statusText}>
            Control Host:Port: {torState.controlUrl || 'Not available'}
          </Text>

          {!torState.isSuccess && (
            <Text style={styles.errorText}>Error: {torState.errorMessage}</Text>
          )}

          <View style={styles.buttonContainer}>
            <Button title="HTTP GET" onPress={httpGet} />
            <Button title="HTTP POST" onPress={httpPost} />
            <Button title="HTTP PUT" onPress={httpPut} />
            <Button title="HTTP DELETE" onPress={httpDelete} />
          </View>

          <View style={styles.clearButtonContainer}>
            <Button
              title="Clear All Responses"
              onPress={clearAllResults}
              color="#ff6347"
            />
          </View>

          {renderResult('GET Response', getResult)}
          {renderResult('POST Response', postResult)}
          {renderResult('PUT Response', putResult)}
          {renderResult('DELETE Response', deleteResult)}
        </View>
      </ScrollView>
    </SafeAreaView>
  );
}

const styles = StyleSheet.create({
  scrollContainer: {
    flex: 1,
  },
  container: {
    flex: 1,
    alignItems: 'center',
    justifyContent: 'center',
    padding: 20,
    paddingTop: Platform.OS === 'ios' ? 50 : 20, // Add extra padding for iOS notch
  },
  headerText: {
    fontSize: 24,
    fontWeight: 'bold',
    marginBottom: 20,
  },
  statusText: {
    fontSize: 16,
    marginVertical: 8,
    textAlign: 'center',
  },
  buttonContainer: {
    flexDirection: 'row',
    flexWrap: 'wrap',
    justifyContent: 'center',
    marginVertical: 16,
    gap: 10,
  },
  resultContainer: {
    width: '100%',
    marginVertical: 10,
    padding: 10,
    borderWidth: 1,
    borderColor: '#ccc',
    borderRadius: 5,
  },
  resultTitle: {
    fontSize: 18,
    fontWeight: 'bold',
    marginBottom: 8,
  },
  resultText: {
    fontSize: 14,
    marginBottom: 4,
  },
  errorText: {
    fontSize: 14,
    color: 'red',
    marginVertical: 4,
  },
  clearButtonContainer: {
    marginVertical: 10,
    width: '100%',
  },
  responseBody: {
    maxHeight: 200,
    marginTop: 10,
    padding: 8,
    backgroundColor: '#f5f5f5',
    borderRadius: 4,
  },
});
