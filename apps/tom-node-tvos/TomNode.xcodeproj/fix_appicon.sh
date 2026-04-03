#!/bin/bash

echo "========================================="
echo "🛠️  Correction AppIcon tvOS"
echo "========================================="
echo ""

# Trouver le répertoire du projet
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$SCRIPT_DIR/.."

echo "📂 Recherche dans: $PROJECT_ROOT"
echo ""

FIXED=0

# Trouver et corriger tous les fichiers project.pbxproj
while IFS= read -r -d '' file; do
    echo "📝 Traitement: $file"
    
    # Vérifier si le fichier contient l'erreur
    if grep -q "App Icon & Top Shelf Image" "$file"; then
        # Faire un backup
        cp "$file" "$file.backup"
        
        # Remplacer
        sed -i '' 's/App Icon & Top Shelf Image/AppIcon/g' "$file"
        
        if [ $? -eq 0 ]; then
            echo "   ✅ Corrigé (backup: $file.backup)"
            FIXED=$((FIXED + 1))
        else
            echo "   ❌ Erreur lors de la correction"
            mv "$file.backup" "$file"
        fi
    else
        echo "   ℹ️  Pas de modification nécessaire"
    fi
    echo ""
done < <(find "$PROJECT_ROOT" -name "project.pbxproj" -print0)

# Trouver et corriger tous les fichiers .xcconfig
while IFS= read -r -d '' file; do
    echo "📝 Traitement: $file"
    
    if grep -q "App Icon & Top Shelf Image" "$file"; then
        cp "$file" "$file.backup"
        sed -i '' 's/App Icon & Top Shelf Image/AppIcon/g' "$file"
        
        if [ $? -eq 0 ]; then
            echo "   ✅ Corrigé (backup: $file.backup)"
            FIXED=$((FIXED + 1))
        else
            echo "   ❌ Erreur lors de la correction"
            mv "$file.backup" "$file"
        fi
    else
        echo "   ℹ️  Pas de modification nécessaire"
    fi
    echo ""
done < <(find "$PROJECT_ROOT" -name "*.xcconfig" -print0)

echo "========================================="
if [ $FIXED -gt 0 ]; then
    echo "✅ $FIXED fichier(s) corrigé(s)"
    echo ""
    echo "🔄 Actions à faire maintenant:"
    echo "   1. Fermez Xcode si ouvert"
    echo "   2. Rouvrez votre projet"
    echo "   3. Clean Build Folder (Shift+Cmd+K)"
    echo "   4. Build (Cmd+B)"
else
    echo "⚠️  Aucune correction automatique possible"
    echo ""
    echo "💡 Faites manuellement dans Xcode:"
    echo "   1. Sélectionnez votre target tvOS"
    echo "   2. Build Settings → Chercher 'ASSETCATALOG_COMPILER_APPICON_NAME'"
    echo "   3. Changez la valeur en 'AppIcon'"
    echo "   4. Ou supprimez complètement la ligne"
fi
echo "========================================="
