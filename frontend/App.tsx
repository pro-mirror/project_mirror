import { StatusBar } from 'expo-status-bar';
import { NavigationContainer } from '@react-navigation/native';
import { createBottomTabNavigator } from '@react-navigation/bottom-tabs';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { GestureHandlerRootView } from 'react-native-gesture-handler';
import { StyleSheet } from 'react-native';
import { Ionicons } from '@expo/vector-icons';

import HomeScreen from './src/screens/HomeScreen';
import ChronicleScreen from './src/screens/ChronicleScreen';
import { theme } from './src/theme';

const Tab = createBottomTabNavigator();
const queryClient = new QueryClient();

export default function App() {
  return (
    <GestureHandlerRootView style={styles.container}>
      <QueryClientProvider client={queryClient}>
        <NavigationContainer>
          <StatusBar style="light" />
          <Tab.Navigator
            screenOptions={({ route }) => ({
              tabBarStyle: {
                backgroundColor: theme.colors.background,
                borderTopColor: theme.colors.border,
              },
              tabBarActiveTintColor: theme.colors.accent,
              tabBarInactiveTintColor: theme.colors.textSecondary,
              tabBarShowLabel: false,
              headerStyle: {
                backgroundColor: theme.colors.background,
              },
              headerTintColor: theme.colors.text,
              tabBarIcon: ({ focused, color, size }) => {
                let iconName: keyof typeof Ionicons.glyphMap;

                if (route.name === 'Home') {
                  iconName = focused ? 'chatbubble' : 'chatbubble-outline';
                } else if (route.name === 'Chronicle') {
                  iconName = focused ? 'book' : 'book-outline';
                } else {
                  iconName = 'help-outline';
                }

                return <Ionicons name={iconName} size={size} color={color} />;
              },
            })}
          >
            <Tab.Screen 
              name="Home" 
              component={HomeScreen}
              options={{ title: '対話' }}
            />
            <Tab.Screen 
              name="Chronicle" 
              component={ChronicleScreen}
              options={{ title: '記憶' }}
            />
          </Tab.Navigator>
        </NavigationContainer>
      </QueryClientProvider>
    </GestureHandlerRootView>
  );
}

const styles = StyleSheet.create({
  container: {
    flex: 1,
  },
});
