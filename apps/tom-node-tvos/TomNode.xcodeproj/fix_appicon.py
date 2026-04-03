#!/usr/bin/env python3
"""
Script pour corriger l'erreur d'AppIcon tvOS
Remplace "App Icon & Top Shelf Image" par "AppIcon"
"""

import os
import re
import sys
from pathlib import Path

def fix_pbxproj_file(file_path):
    """Corrige le fichier project.pbxproj"""
    try:
        with open(file_path, 'r', encoding='utf-8') as f:
            content = f.read()
        
        original_content = content
        
        # Remplacer toutes les occurrences
        content = content.replace('"App Icon & Top Shelf Image"', '"AppIcon"')
        content = content.replace("'App Icon & Top Shelf Image'", "'AppIcon'")
        content = re.sub(r'ASSETCATALOG_COMPILER_APPICON_NAME\s*=\s*"App Icon & Top Shelf Image"', 
                        'ASSETCATALOG_COMPILER_APPICON_NAME = AppIcon', content)
        
        if content != original_content:
            with open(file_path, 'w', encoding='utf-8') as f:
                f.write(content)
            print(f"✅ Corrigé: {file_path}")
            return True
        else:
            print(f"ℹ️  Aucune modification nécessaire: {file_path}")
            return False
    except Exception as e:
        print(f"❌ Erreur sur {file_path}: {e}")
        return False

def fix_xcconfig_file(file_path):
    """Corrige les fichiers .xcconfig"""
    try:
        with open(file_path, 'r', encoding='utf-8') as f:
            content = f.read()
        
        original_content = content
        
        content = content.replace('App Icon & Top Shelf Image', 'AppIcon')
        
        if content != original_content:
            with open(file_path, 'w', encoding='utf-8') as f:
                f.write(content)
            print(f"✅ Corrigé: {file_path}")
            return True
        else:
            print(f"ℹ️  Aucune modification nécessaire: {file_path}")
            return False
    except Exception as e:
        print(f"❌ Erreur sur {file_path}: {e}")
        return False

def find_and_fix_files(root_dir):
    """Trouve et corrige tous les fichiers pertinents"""
    root_path = Path(root_dir)
    fixed_count = 0
    
    print("🔍 Recherche des fichiers à corriger...")
    print()
    
    # Chercher les fichiers project.pbxproj
    for pbxproj in root_path.rglob("project.pbxproj"):
        print(f"📝 Traitement de: {pbxproj}")
        if fix_pbxproj_file(pbxproj):
            fixed_count += 1
    
    # Chercher les fichiers .xcconfig
    for xcconfig in root_path.rglob("*.xcconfig"):
        print(f"📝 Traitement de: {xcconfig}")
        if fix_xcconfig_file(xcconfig):
            fixed_count += 1
    
    print()
    print("="*50)
    if fixed_count > 0:
        print(f"✅ {fixed_count} fichier(s) corrigé(s)")
        print()
        print("🔄 Actions à faire dans Xcode:")
        print("   1. Fermez Xcode si ouvert")
        print("   2. Rouvrez votre projet")
        print("   3. Clean Build Folder (Shift+Cmd+K)")
        print("   4. Build (Cmd+B)")
    else:
        print("⚠️  Aucun fichier n'a été modifié")
        print()
        print("💡 Solutions alternatives:")
        print("   1. Dans Xcode: Build Settings → Chercher 'ASSETCATALOG_COMPILER_APPICON_NAME'")
        print("   2. Changer 'App Icon & Top Shelf Image' en 'AppIcon'")
        print("   3. Ou supprimez complètement la valeur")
    print("="*50)

if __name__ == "__main__":
    if len(sys.argv) > 1:
        root_dir = sys.argv[1]
    else:
        # Chercher depuis le répertoire du script
        root_dir = Path(__file__).parent.parent
    
    print("="*50)
    print("🛠️  Correction de l'erreur AppIcon tvOS")
    print("="*50)
    print()
    print(f"📂 Répertoire de recherche: {root_dir}")
    print()
    
    find_and_fix_files(root_dir)
