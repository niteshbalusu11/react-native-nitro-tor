import { useEffect, useMemo, useRef, useState } from 'react';
import {
  Animated,
  Pressable,
  StatusBar,
  Text,
  View,
  StyleSheet,
  ScrollView,
  useColorScheme,
} from 'react-native';
import { SafeAreaProvider, SafeAreaView } from 'react-native-safe-area-context';
import {
  DocumentDirectoryPath,
  exists,
  mkdir,
} from '@dr.pogodin/react-native-fs';
import { Tor, type TorStatus } from 'react-native-nitro-tor';

// Constants
const TOR_DATA_PATH = `${DocumentDirectoryPath}/tor_data`;
const GET_URL = 'https://httpbin.org/get';
const ONION_GET_URL =
  'http://duckduckgogg42xjoc72x3sjasowoarfbgcmvfimaftt6twagswzczad.onion';
const POST_URL = 'http://httpbin.org/post';
const PUT_URL = 'http://httpbin.org/put';
const DELETE_URL = 'http://httpbin.org/delete';

interface Theme {
  background: string;
  surface: string;
  surfaceRaised: string;
  text: string;
  muted: string;
  subtle: string;
  border: string;
  accent: string;
  accentSoft: string;
  success: string;
  successSoft: string;
  warning: string;
  danger: string;
  dangerSoft: string;
  shadow: string;
}

const LIGHT_THEME: Theme = {
  background: '#F7F5F9',
  surface: '#FFFFFF',
  surfaceRaised: '#F0ECF4',
  text: '#19151E',
  muted: '#6E6874',
  subtle: '#938C9A',
  border: '#E4DEE8',
  accent: '#7047A8',
  accentSoft: '#EEE6F8',
  success: '#197A5A',
  successSoft: '#E0F3EB',
  warning: '#B06B20',
  danger: '#B73D55',
  dangerSoft: '#FAE7EB',
  shadow: '#1B1027',
};

const DARK_THEME: Theme = {
  background: '#100D13',
  surface: '#19151E',
  surfaceRaised: '#211B27',
  text: '#F7F2FA',
  muted: '#A79FAC',
  subtle: '#7D7484',
  border: '#332B39',
  accent: '#B794E7',
  accentSoft: '#302441',
  success: '#66D4AB',
  successSoft: '#19372E',
  warning: '#E9A65B',
  danger: '#F08CA0',
  dangerSoft: '#42232D',
  shadow: '#000000',
};

interface TorState {
  isSuccess: boolean | undefined;
  errorMessage: string | undefined;
  onionUrl: string | undefined;
  socksAddress: string | undefined;
}

interface RequestResult {
  status: number;
  body: string;
  error: string;
}

export default function TorApp() {
  const systemScheme = useColorScheme();
  const [themeOverride, setThemeOverride] = useState<'light' | 'dark' | null>(
    null,
  );
  const isDark = (themeOverride ?? systemScheme) === 'dark';
  const theme = isDark ? DARK_THEME : LIGHT_THEME;
  const styles = useMemo(() => createStyles(theme), [theme]);

  const [torState, setTorState] = useState<TorState>({
    isSuccess: undefined,
    errorMessage: undefined,
    onionUrl: undefined,
    socksAddress: undefined,
  });
  const [getResult, setGetResult] = useState<RequestResult | null>(null);
  const [onionGetResult, setOnionGetResult] = useState<RequestResult | null>(
    null,
  );
  const [postResult, setPostResult] = useState<RequestResult | null>(null);
  const [putResult, setPutResult] = useState<RequestResult | null>(null);
  const [deleteResult, setDeleteResult] = useState<RequestResult | null>(null);
  const introOpacity = useRef(new Animated.Value(0)).current;
  const introOffset = useRef(new Animated.Value(12)).current;
  const pulseOpacity = useRef(new Animated.Value(0.45)).current;
  const detailsOpacity = useRef(new Animated.Value(0)).current;

  useEffect(() => {
    const entrance = Animated.parallel([
      Animated.timing(introOpacity, {
        toValue: 1,
        duration: 420,
        useNativeDriver: true,
      }),
      Animated.timing(introOffset, {
        toValue: 0,
        duration: 420,
        useNativeDriver: true,
      }),
    ]);
    const pulse = Animated.loop(
      Animated.sequence([
        Animated.timing(pulseOpacity, {
          toValue: 1,
          duration: 850,
          useNativeDriver: true,
        }),
        Animated.timing(pulseOpacity, {
          toValue: 0.45,
          duration: 850,
          useNativeDriver: true,
        }),
      ]),
    );

    entrance.start();
    pulse.start();

    return () => {
      entrance.stop();
      pulse.stop();
    };
  }, [introOffset, introOpacity, pulseOpacity]);

  useEffect(() => {
    Animated.timing(detailsOpacity, {
      toValue: torState.isSuccess ? 1 : 0,
      duration: 280,
      useNativeDriver: true,
    }).start();
  }, [detailsOpacity, torState.isSuccess]);

  const clearAllResults = () => {
    setGetResult(null);
    setOnionGetResult(null);
    setPostResult(null);
    setPutResult(null);
    setDeleteResult(null);
  };

  useEffect(() => {
    const unsubscribe = Tor.daemon.subscribe((status: TorStatus) => {
      if (status.state === 'failed') {
        setTorState(prev => ({
          ...prev,
          errorMessage: status.error.message,
          isSuccess: false,
        }));
      }
    });
    const initTor = async () => {
      try {
        // Ensure directory exists
        await ensureDataDirectory();

        console.log('Ensuring data directory');

        const status = await Tor.daemon.start({
          dataDirectory: TOR_DATA_PATH,
          socksPort: 9050,
          bootstrapTimeoutMs: 120000,
        });
        const hiddenService = await Tor.hiddenServices.create({
          virtualPort: 80,
          targetPort: 8080,
        });

        setTorState({
          isSuccess: true,
          errorMessage: undefined,
          onionUrl: hiddenService.onionAddress,
          socksAddress: status.socksAddress,
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
      unsubscribe();
      Tor.daemon.stop().catch(console.error);
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
      const result = await Tor.http.request({
        url: GET_URL,
        timeoutMs: 20000,
      });
      console.log('httpGet result', result);
      setGetResult({
        status: result.statusCode,
        body: result.body,
        error: '',
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

  const onionHttpGet = async () => {
    try {
      const result = await Tor.http.request({
        url: ONION_GET_URL,
        timeoutMs: 30000,
      });
      console.log('onionHttpGet result', result);
      setOnionGetResult({
        status: result.statusCode,
        body: result.body,
        error: '',
      });
    } catch (err: any) {
      console.error('onionHttpGet error', err);
      setOnionGetResult({
        status: 0,
        body: '',
        error: err.message,
      });
    }
  };

  const httpPost = async () => {
    try {
      const result = await Tor.http.request({
        url: POST_URL,
        method: 'POST',
        body: '{"test":"data"}',
        headers: { 'Content-Type': 'application/json' },
        timeoutMs: 20000,
      });
      console.log('http post result', result);
      setPostResult({
        status: result.statusCode,
        body: result.body,
        error: '',
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
      const result = await Tor.http.request({
        url: PUT_URL,
        method: 'PUT',
        body: '{"updated":"value"}',
        headers: { 'Content-Type': 'application/json' },
        timeoutMs: 20000,
      });
      console.log('http put result', result);
      setPutResult({
        status: result.statusCode,
        body: result.body,
        error: '',
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
      const result = await Tor.http.request({
        url: DELETE_URL,
        method: 'DELETE',
        headers: { 'Content-Type': 'application/json' },
        timeoutMs: 20000,
      });
      console.log('http delete result', result);
      setDeleteResult({
        status: result.statusCode,
        body: result.body,
        error: '',
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

  const requestActions = [
    {
      key: 'get',
      method: 'GET',
      title: 'Public request',
      endpoint: GET_URL,
      onPress: httpGet,
    },
    {
      key: 'onion-get',
      method: 'GET',
      title: 'Onion request',
      endpoint: ONION_GET_URL,
      onPress: onionHttpGet,
    },
    {
      key: 'post',
      method: 'POST',
      title: 'Create resource',
      endpoint: POST_URL,
      onPress: httpPost,
    },
    {
      key: 'put',
      method: 'PUT',
      title: 'Update resource',
      endpoint: PUT_URL,
      onPress: httpPut,
    },
    {
      key: 'delete',
      method: 'DELETE',
      title: 'Delete resource',
      endpoint: DELETE_URL,
      onPress: httpDelete,
    },
  ];

  const responses = [
    { key: 'get', title: 'Public GET', result: getResult },
    { key: 'onion-get', title: 'Onion GET', result: onionGetResult },
    { key: 'post', title: 'POST', result: postResult },
    { key: 'put', title: 'PUT', result: putResult },
    { key: 'delete', title: 'DELETE', result: deleteResult },
  ].filter(response => response.result !== null);

  const status =
    torState.isSuccess === undefined
      ? {
          title: 'Connecting',
          detail: 'Building a private circuit',
          color: theme.warning,
        }
      : torState.isSuccess
      ? {
          title: 'Tor is ready',
          detail: 'Traffic can be routed securely',
          color: theme.success,
        }
      : {
          title: 'Connection failed',
          detail: 'The Tor service needs attention',
          color: theme.danger,
        };

  const renderResult = (title: string, result: RequestResult) => {
    const hasError = Boolean(result.error) || result.status === 0;

    return (
      <View style={styles.resultContainer}>
        <View style={styles.resultHeader}>
          <Text style={styles.resultTitle}>{title}</Text>
          <View
            style={[
              styles.statusBadge,
              hasError ? styles.statusBadgeError : styles.statusBadgeSuccess,
            ]}
          >
            <Text
              style={[
                styles.statusBadgeText,
                hasError
                  ? styles.statusBadgeTextError
                  : styles.statusBadgeTextSuccess,
              ]}
            >
              {hasError ? 'FAILED' : result.status}
            </Text>
          </View>
        </View>
        {result.error ? (
          <Text style={styles.resultError}>{result.error}</Text>
        ) : (
          <ScrollView
            nestedScrollEnabled
            style={styles.responseBody}
            contentContainerStyle={styles.responseBodyContent}
          >
            <Text selectable style={styles.responseText}>
              {result.body || 'The response body is empty.'}
            </Text>
          </ScrollView>
        )}
      </View>
    );
  };

  return (
    <SafeAreaProvider style={styles.safeArea}>
      <SafeAreaView edges={['top', 'bottom']} style={styles.safeArea}>
        <StatusBar barStyle={isDark ? 'light-content' : 'dark-content'} />
        <ScrollView
          style={styles.scrollView}
          contentContainerStyle={styles.container}
          showsVerticalScrollIndicator={false}
        >
          <Animated.View
            style={[
              styles.intro,
              {
                opacity: introOpacity,
                transform: [{ translateY: introOffset }],
              },
            ]}
          >
            <View style={styles.headerRow}>
              <View style={styles.brandGroup}>
                <View style={styles.brandMark} accessibilityElementsHidden>
                  <View style={styles.brandMarkRing}>
                    <View style={styles.brandMarkCore} />
                  </View>
                </View>
                <View>
                  <Text style={styles.eyebrow}>REACT NATIVE</Text>
                  <Text style={styles.brandTitle}>Nitro Tor</Text>
                </View>
              </View>
              <Pressable
                accessibilityRole="button"
                accessibilityLabel={`Switch to ${
                  isDark ? 'light' : 'dark'
                } mode`}
                hitSlop={8}
                onPress={() => setThemeOverride(isDark ? 'light' : 'dark')}
                style={({ pressed }) => [
                  styles.themeButton,
                  pressed && styles.themeButtonPressed,
                ]}
              >
                <Text style={styles.themeButtonIcon}>
                  {isDark ? '☀︎' : '☾'}
                </Text>
              </Pressable>
            </View>
            <Text style={styles.introText}>
              Start Tor, inspect the circuit, and send requests through the
              local SOCKS proxy.
            </Text>
          </Animated.View>

          <View style={styles.statusSection}>
            <Text style={styles.sectionLabel}>TOR DAEMON</Text>
            <View style={styles.statusHeader}>
              <View style={styles.statusTitleRow}>
                <Animated.View
                  style={[
                    styles.statusDot,
                    { backgroundColor: status.color, opacity: pulseOpacity },
                  ]}
                />
                <Text style={styles.statusTitle}>{status.title}</Text>
              </View>
              <Text style={styles.statusDetail}>{status.detail}</Text>
            </View>

            {torState.isSuccess ? (
              <Animated.View
                style={[styles.connectionDetails, { opacity: detailsOpacity }]}
              >
                <View style={styles.detailRow}>
                  <Text style={styles.detailLabel}>SOCKS proxy</Text>
                  <Text selectable style={styles.detailValue}>
                    {torState.socksAddress}
                  </Text>
                </View>
                <View style={styles.detailDivider} />
                <View style={styles.detailRow}>
                  <Text style={styles.detailLabel}>Onion service</Text>
                </View>
                <View style={styles.detailRowStacked}>
                  <Text selectable style={styles.onionValue}>
                    {torState.onionUrl}
                  </Text>
                </View>
              </Animated.View>
            ) : torState.isSuccess === false ? (
              <View style={styles.errorPanel}>
                <Text style={styles.errorPanelText}>
                  {torState.errorMessage || 'Tor could not start.'}
                </Text>
              </View>
            ) : (
              <View style={styles.loadingTrack}>
                <View style={styles.loadingProgress} />
              </View>
            )}
          </View>

          <View style={styles.sectionHeadingRow}>
            <View>
              <Text style={styles.sectionLabel}>REQUEST LAB</Text>
              <Text style={styles.sectionTitle}>Route a request</Text>
            </View>
            <Text style={styles.sectionCount}>
              {requestActions.length} routes
            </Text>
          </View>
          <Text style={styles.sectionDescription}>
            Each request is dispatched through the embedded Tor client.
          </Text>

          <View style={styles.actionList}>
            {requestActions.map((action, index) => (
              <Pressable
                accessibilityRole="button"
                accessibilityLabel={`${action.method} ${action.title}`}
                disabled={torState.isSuccess !== true}
                key={action.key}
                onPress={action.onPress}
                style={({ pressed }) => [
                  styles.actionRow,
                  index < requestActions.length - 1 && styles.actionDivider,
                  torState.isSuccess !== true && styles.actionDisabled,
                  pressed && styles.actionPressed,
                ]}
              >
                <View style={styles.methodBadge}>
                  <Text style={styles.methodBadgeText}>{action.method}</Text>
                </View>
                <View style={styles.actionCopy}>
                  <Text style={styles.actionTitle}>{action.title}</Text>
                  <Text numberOfLines={1} style={styles.endpointText}>
                    {action.endpoint}
                  </Text>
                </View>
                <Text style={styles.actionArrow}>›</Text>
              </Pressable>
            ))}
          </View>

          <View style={styles.responsesHeader}>
            <View>
              <Text style={styles.sectionLabel}>OUTPUT</Text>
              <Text style={styles.sectionTitle}>Responses</Text>
            </View>
            {responses.length > 0 && (
              <Pressable
                accessibilityRole="button"
                onPress={clearAllResults}
                style={({ pressed }) => [
                  styles.clearButton,
                  pressed && styles.clearButtonPressed,
                ]}
              >
                <Text style={styles.clearButtonText}>Clear all</Text>
              </Pressable>
            )}
          </View>

          {responses.length > 0 ? (
            <View style={styles.resultList}>
              {responses.map(response => (
                <View key={response.key}>
                  {renderResult(response.title, response.result!)}
                </View>
              ))}
            </View>
          ) : (
            <View style={styles.emptyState}>
              <Text style={styles.emptyGlyph}>{'{ }'}</Text>
              <View style={styles.emptyCopy}>
                <Text style={styles.emptyTitle}>No responses yet</Text>
                <Text style={styles.emptyText}>
                  Run a route above to inspect its status and response body.
                </Text>
              </View>
            </View>
          )}

          <View style={styles.footer}>
            <View style={styles.footerStatus}>
              <View style={styles.footerDot} />
              <Text style={styles.footerText}>Embedded Tor runtime</Text>
            </View>
            <Text style={styles.footerVersion}>0.4.9.11</Text>
          </View>
        </ScrollView>
      </SafeAreaView>
    </SafeAreaProvider>
  );
}

const createStyles = (theme: Theme) =>
  StyleSheet.create({
    safeArea: {
      flex: 1,
      backgroundColor: theme.background,
    },
    scrollView: {
      flex: 1,
    },
    container: {
      width: '100%',
      maxWidth: 720,
      alignSelf: 'center',
      paddingHorizontal: 22,
      paddingTop: 24,
      paddingBottom: 44,
    },
    intro: {
      marginBottom: 30,
    },
    headerRow: {
      flexDirection: 'row',
      alignItems: 'center',
      justifyContent: 'space-between',
    },
    brandGroup: {
      flexDirection: 'row',
      alignItems: 'center',
      gap: 12,
    },
    brandMark: {
      width: 48,
      height: 48,
      borderRadius: 16,
      backgroundColor: theme.accentSoft,
      alignItems: 'center',
      justifyContent: 'center',
    },
    brandMarkRing: {
      width: 25,
      height: 29,
      borderRadius: 14,
      borderWidth: 2,
      borderColor: theme.accent,
      alignItems: 'center',
      justifyContent: 'center',
    },
    brandMarkCore: {
      width: 9,
      height: 13,
      borderRadius: 7,
      backgroundColor: theme.accent,
    },
    eyebrow: {
      color: theme.muted,
      fontSize: 10,
      fontWeight: '700',
      letterSpacing: 1.7,
      marginBottom: 2,
    },
    brandTitle: {
      color: theme.text,
      fontSize: 25,
      fontWeight: '700',
      letterSpacing: -0.7,
    },
    themeButton: {
      width: 42,
      height: 42,
      borderRadius: 14,
      borderWidth: 1,
      borderColor: theme.border,
      backgroundColor: theme.surface,
      alignItems: 'center',
      justifyContent: 'center',
    },
    themeButtonPressed: {
      opacity: 0.68,
      transform: [{ scale: 0.96 }],
    },
    themeButtonIcon: {
      color: theme.text,
      fontSize: 19,
      lineHeight: 22,
    },
    introText: {
      color: theme.muted,
      fontSize: 15,
      lineHeight: 22,
      marginTop: 18,
      maxWidth: 480,
    },
    sectionLabel: {
      color: theme.subtle,
      fontSize: 10,
      fontWeight: '700',
      letterSpacing: 1.6,
    },
    statusSection: {
      backgroundColor: theme.surface,
      borderWidth: 1,
      borderColor: theme.border,
      borderRadius: 24,
      padding: 20,
      shadowColor: theme.shadow,
      shadowOffset: { width: 0, height: 10 },
      shadowOpacity: 0.08,
      shadowRadius: 24,
      elevation: 3,
    },
    statusHeader: {
      marginTop: 12,
    },
    statusTitleRow: {
      flexDirection: 'row',
      alignItems: 'center',
      gap: 9,
    },
    statusDot: {
      width: 9,
      height: 9,
      borderRadius: 5,
    },
    statusTitle: {
      color: theme.text,
      fontSize: 23,
      fontWeight: '700',
      letterSpacing: -0.5,
    },
    statusDetail: {
      color: theme.muted,
      fontSize: 14,
      marginTop: 5,
      marginLeft: 18,
    },
    connectionDetails: {
      backgroundColor: theme.surfaceRaised,
      borderRadius: 16,
      marginTop: 20,
      paddingHorizontal: 15,
      paddingVertical: 4,
    },
    detailRow: {
      flexDirection: 'row',
      justifyContent: 'space-between',
      alignItems: 'center',
      gap: 18,
      paddingVertical: 13,
    },
    detailRowStacked: {
      paddingVertical: 13,
    },
    detailDivider: {
      height: StyleSheet.hairlineWidth,
      backgroundColor: theme.border,
    },
    detailLabel: {
      color: theme.muted,
      fontSize: 12,
      fontWeight: '600',
    },
    detailValue: {
      flexShrink: 1,
      color: theme.text,
      fontSize: 12,
      fontVariant: ['tabular-nums'],
      textAlign: 'right',
    },
    onionValue: {
      color: theme.text,
      fontSize: 11,
      lineHeight: 17,
      marginTop: 6,
    },
    loadingTrack: {
      height: 3,
      borderRadius: 2,
      backgroundColor: theme.surfaceRaised,
      marginTop: 22,
      overflow: 'hidden',
    },
    loadingProgress: {
      width: '46%',
      height: '100%',
      borderRadius: 2,
      backgroundColor: theme.accent,
    },
    errorPanel: {
      backgroundColor: theme.dangerSoft,
      borderRadius: 14,
      marginTop: 18,
      padding: 14,
    },
    errorPanelText: {
      color: theme.danger,
      fontSize: 13,
      lineHeight: 19,
    },
    sectionHeadingRow: {
      flexDirection: 'row',
      alignItems: 'flex-end',
      justifyContent: 'space-between',
      marginTop: 38,
    },
    sectionTitle: {
      color: theme.text,
      fontSize: 22,
      fontWeight: '700',
      letterSpacing: -0.4,
      marginTop: 6,
    },
    sectionCount: {
      color: theme.subtle,
      fontSize: 12,
      marginBottom: 3,
    },
    sectionDescription: {
      color: theme.muted,
      fontSize: 13,
      lineHeight: 19,
      marginTop: 8,
      marginBottom: 15,
    },
    actionList: {
      backgroundColor: theme.surface,
      borderWidth: 1,
      borderColor: theme.border,
      borderRadius: 20,
      overflow: 'hidden',
    },
    actionRow: {
      minHeight: 75,
      flexDirection: 'row',
      alignItems: 'center',
      paddingHorizontal: 15,
      paddingVertical: 13,
      backgroundColor: theme.surface,
    },
    actionDivider: {
      borderBottomWidth: StyleSheet.hairlineWidth,
      borderBottomColor: theme.border,
    },
    actionDisabled: {
      opacity: 0.44,
    },
    actionPressed: {
      backgroundColor: theme.surfaceRaised,
      transform: [{ scale: 0.995 }],
    },
    methodBadge: {
      minWidth: 53,
      height: 31,
      borderRadius: 10,
      backgroundColor: theme.accentSoft,
      alignItems: 'center',
      justifyContent: 'center',
      paddingHorizontal: 8,
    },
    methodBadgeText: {
      color: theme.accent,
      fontSize: 10,
      fontWeight: '800',
      letterSpacing: 0.5,
    },
    actionCopy: {
      flex: 1,
      marginLeft: 13,
      marginRight: 8,
    },
    actionTitle: {
      color: theme.text,
      fontSize: 14,
      fontWeight: '600',
      marginBottom: 4,
    },
    endpointText: {
      color: theme.muted,
      fontSize: 11,
    },
    actionArrow: {
      color: theme.subtle,
      fontSize: 25,
      fontWeight: '300',
      marginTop: -2,
    },
    responsesHeader: {
      flexDirection: 'row',
      alignItems: 'flex-end',
      justifyContent: 'space-between',
      marginTop: 38,
      marginBottom: 15,
    },
    clearButton: {
      borderRadius: 10,
      paddingHorizontal: 11,
      paddingVertical: 8,
      marginBottom: -2,
    },
    clearButtonPressed: {
      backgroundColor: theme.dangerSoft,
    },
    clearButtonText: {
      color: theme.danger,
      fontSize: 12,
      fontWeight: '600',
    },
    resultList: {
      gap: 12,
    },
    resultContainer: {
      backgroundColor: theme.surface,
      borderWidth: 1,
      borderColor: theme.border,
      borderRadius: 18,
      padding: 16,
    },
    resultHeader: {
      flexDirection: 'row',
      alignItems: 'center',
      justifyContent: 'space-between',
    },
    resultTitle: {
      color: theme.text,
      fontSize: 15,
      fontWeight: '700',
    },
    statusBadge: {
      borderRadius: 8,
      paddingHorizontal: 8,
      paddingVertical: 5,
    },
    statusBadgeSuccess: {
      backgroundColor: theme.successSoft,
    },
    statusBadgeError: {
      backgroundColor: theme.dangerSoft,
    },
    statusBadgeText: {
      fontSize: 9,
      fontWeight: '800',
      letterSpacing: 0.7,
    },
    statusBadgeTextSuccess: {
      color: theme.success,
    },
    statusBadgeTextError: {
      color: theme.danger,
    },
    resultError: {
      color: theme.danger,
      fontSize: 13,
      lineHeight: 19,
      marginTop: 13,
    },
    responseBody: {
      maxHeight: 220,
      backgroundColor: theme.surfaceRaised,
      borderRadius: 12,
      marginTop: 13,
    },
    responseBodyContent: {
      padding: 13,
    },
    responseText: {
      color: theme.text,
      fontSize: 11,
      lineHeight: 17,
    },
    emptyState: {
      flexDirection: 'row',
      alignItems: 'center',
      borderTopWidth: 1,
      borderBottomWidth: 1,
      borderColor: theme.border,
      paddingVertical: 22,
    },
    emptyGlyph: {
      color: theme.accent,
      fontSize: 19,
      fontWeight: '700',
      width: 52,
    },
    emptyCopy: {
      flex: 1,
    },
    emptyTitle: {
      color: theme.text,
      fontSize: 14,
      fontWeight: '600',
      marginBottom: 4,
    },
    emptyText: {
      color: theme.muted,
      fontSize: 12,
      lineHeight: 18,
    },
    footer: {
      flexDirection: 'row',
      alignItems: 'center',
      justifyContent: 'space-between',
      marginTop: 38,
      paddingTop: 18,
      borderTopWidth: StyleSheet.hairlineWidth,
      borderTopColor: theme.border,
    },
    footerStatus: {
      flexDirection: 'row',
      alignItems: 'center',
      gap: 7,
    },
    footerDot: {
      width: 6,
      height: 6,
      borderRadius: 3,
      backgroundColor: theme.success,
    },
    footerText: {
      color: theme.muted,
      fontSize: 11,
    },
    footerVersion: {
      color: theme.subtle,
      fontSize: 11,
      fontVariant: ['tabular-nums'],
    },
  });
