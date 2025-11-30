# Add project specific ProGuard rules here.

# Keep UniFFI generated classes
-keep class uniffi.toboggan.** { *; }

# Keep JNA classes
-keep class com.sun.jna.** { *; }
-keepclassmembers class * extends com.sun.jna.** { public *; }
