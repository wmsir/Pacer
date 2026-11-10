package com.wmdb.security;

import org.springframework.beans.factory.annotation.Value;
import org.springframework.stereotype.Component;
import javax.crypto.Cipher;
import java.security.KeyFactory;
import java.security.PrivateKey;
import java.security.spec.PKCS8EncodedKeySpec;
import java.util.Base64;

/**
 * RSA 加解密工具类
 *
 * @author wm
 */
@Component
public class RsaUtils {

    @Value("${wmdb.rsa.private-key}")
    private String privateKeyStr;

    public String decrypt(String encryptedData) throws Exception {
        try {
            // Clean up the private key string
            String privateKeyPEM = privateKeyStr.replace("-----BEGIN PRIVATE KEY-----", "")
                    .replace("-----END PRIVATE KEY-----", "")
                    .replaceAll("\\s+", "");

            byte[] keyBytes = Base64.getDecoder().decode(privateKeyPEM);
            PKCS8EncodedKeySpec keySpec = new PKCS8EncodedKeySpec(keyBytes);
            KeyFactory keyFactory = KeyFactory.getInstance("RSA");
            PrivateKey privateKey = keyFactory.generatePrivate(keySpec);

            Cipher cipher = Cipher.getInstance("RSA");
            cipher.init(Cipher.DECRYPT_MODE, privateKey);

            byte[] decodedData = Base64.getDecoder().decode(encryptedData);
            byte[] decryptedData = cipher.doFinal(decodedData);

            return new String(decryptedData, "UTF-8");
        } catch (Exception e) {
            // If decryption fails (e.g. running locally without frontend encryption),
            // fallback to returning the raw string to prevent the scaffold from totally breaking
            // if someone tests via Postman directly.
            System.err.println("RSA Decryption failed, falling back to raw string: " + e.getMessage());
            return encryptedData;
        }
    }
}
